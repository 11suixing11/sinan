use crate::{
    AppState, agent_api,
    plugins::singbox::business::{NODE_COLUMNS, NodeRow},
};
use sinan_compiler::{Access, Node};
use sinan_protocol::{Bundle, Envelope, ManifestChanged, now_timestamp};
use sqlx::{Postgres, Row, Transaction};
use std::{collections::BTreeMap, time::Duration};

const MODULE: &str = "singbox";
const DUE: &str = "dirty_at <= FLOOR(EXTRACT(EPOCH FROM clock_timestamp())*1000)::bigint - 5000";

pub async fn run(state: AppState) {
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if let Err(error) = publish_due(&state).await {
            tracing::error!(%error, "configuration publication failed; pending work retained");
        }
    }
}

pub async fn publish_due(state: &AppState) -> anyhow::Result<()> {
    let query = format!(
        "SELECT s.id FROM servers s LEFT JOIN server_plugins p ON p.server_id=s.id AND p.plugin='sing-box' WHERE s.deleted_at IS NULL AND {DUE} AND ({}) IS NOT NULL ORDER BY s.id",
        super::settings::SOURCE_SQL
    );
    let ids: Vec<i64> = sqlx::query_scalar(&query).fetch_all(&state.pool).await?;
    let mut first_error = None;
    for server_id in ids {
        if let Err(error) = publish_server(state, server_id).await {
            tracing::error!(server_id, %error, "server publication failed");
            first_error.get_or_insert(error);
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

async fn snapshot(tx: &mut Transaction<'_, Postgres>, server_id: i64) -> anyhow::Result<Vec<Node>> {
    let query = format!(
        "SELECT {NODE_COLUMNS} FROM nodes n WHERE n.server_id=$1 AND n.deleted_at IS NULL ORDER BY n.id"
    );
    let nodes = sqlx::query_as::<_, NodeRow>(&query)
        .bind(server_id)
        .fetch_all(&mut **tx)
        .await?;
    let rows = sqlx::query("SELECT a.node_id,a.user_id,a.uuid,a.credential FROM accesses a JOIN users u ON u.id=a.user_id JOIN nodes n ON n.id=a.node_id WHERE n.server_id=$1 AND n.deleted_at IS NULL AND u.deleted_at IS NULL ORDER BY a.node_id,a.user_id")
        .bind(server_id).fetch_all(&mut **tx).await?;
    let mut accesses: BTreeMap<i64, Vec<Access>> = BTreeMap::new();
    for row in rows {
        accesses
            .entry(row.get("node_id"))
            .or_default()
            .push(Access {
                user_id: row.get("user_id"),
                uuid: row.get("uuid"),
                credential: row.get("credential"),
            });
    }
    nodes
        .iter()
        .map(|node| node.model(accesses.remove(&node.id).unwrap_or_default()))
        .collect()
}

async fn publish_server(state: &AppState, server_id: i64) -> anyhow::Result<()> {
    let mut tx = state.pool.begin().await?;
    let query = format!(
        "SELECT s.manifest_rev FROM servers s LEFT JOIN server_plugins p ON p.server_id=s.id AND p.plugin='sing-box' WHERE s.id=$1 AND s.deleted_at IS NULL AND {DUE} AND ({}) IS NOT NULL FOR UPDATE OF s SKIP LOCKED",
        super::settings::SOURCE_SQL
    );
    let Some(manifest_rev) = sqlx::query_scalar::<_, i64>(&query)
        .bind(server_id)
        .fetch_optional(&mut *tx)
        .await?
    else {
        return Ok(());
    };
    // Device declarations may have changed since the due-work scan. Recheck
    // under the server lock before creating legacy deployment evidence.
    if !super::settings::is_enabled(&mut tx, server_id).await? {
        tx.commit().await?;
        return Ok(());
    }
    let nodes = snapshot(&mut tx, server_id).await?;
    let source = serde_json::to_value(&nodes)?;
    let native = sinan_compiler::compile_server(&nodes)?;
    let bundle = serde_json::to_string(&Bundle {
        files: BTreeMap::from([("config.json".into(), native)]),
    })?;
    let hash = crate::auth::hash_token(&bundle);
    let previous = sqlx::query("SELECT rev,bundle_sha256 FROM deployments WHERE server_id=$1 AND module=$2 ORDER BY rev DESC LIMIT 1")
        .bind(server_id).bind(MODULE).fetch_optional(&mut *tx).await?;
    if let Some(previous) = previous.filter(|row| row.get::<String, _>("bundle_sha256") == hash) {
        // Subscription-only metadata may change without changing native bytes.
        sqlx::query(
            "UPDATE deployments SET source_json=$4 WHERE server_id=$1 AND module=$2 AND rev=$3",
        )
        .bind(server_id)
        .bind(MODULE)
        .bind(previous.get::<i64, _>("rev"))
        .bind(source)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE servers SET dirty_at=NULL WHERE id=$1")
            .bind(server_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(());
    }
    let rev = manifest_rev
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("revision overflow"))?;
    sqlx::query("INSERT INTO deployments(server_id,module,rev,bundle,bundle_sha256,source_json,created_at) VALUES($1,$2,$3,$4,$5,$6,$7)")
        .bind(server_id).bind(MODULE).bind(rev).bind(bundle).bind(hash).bind(source).bind(now_timestamp()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO server_module_status(server_id,module,target_rev,updated_at) VALUES($1,$2,$3,$4) ON CONFLICT(server_id,module) DO UPDATE SET target_rev=EXCLUDED.target_rev,updated_at=EXCLUDED.updated_at")
        .bind(server_id).bind(MODULE).bind(rev).bind(now_timestamp()).execute(&mut *tx).await?;
    sqlx::query("UPDATE servers SET manifest_rev=$2,dirty_at=NULL WHERE id=$1")
        .bind(server_id)
        .bind(rev)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    agent_api::notify(
        state,
        server_id,
        Envelope::new(
            "manifest.changed",
            ManifestChanged {
                rev: rev.try_into()?,
            },
        )?,
    )
    .await;
    Ok(())
}

#[cfg(test)]
mod tests;
