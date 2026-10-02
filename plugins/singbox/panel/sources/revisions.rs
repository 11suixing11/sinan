use super::parse::{self, PARSER_VERSION, ParsedBatch};
use crate::error::ApiResult;
use serde_json::{Value, json};
use sqlx::{FromRow, PgConnection};
use std::collections::BTreeMap;

#[derive(FromRow)]
struct Previous {
    identity_key: String,
    name: String,
    present: bool,
    config_sha256: Option<String>,
}

pub(super) async fn save_on(
    connection: &mut PgConnection,
    source_id: i64,
    settings_revision: i64,
    identity_epoch: i64,
    body_sha256: &str,
    batch: ParsedBatch,
    now: i64,
) -> ApiResult<i64> {
    let previous: Vec<Previous> = sqlx::query_as("SELECT n.identity_key,n.name,n.present,v.config_sha256 FROM singbox_external_nodes n LEFT JOIN singbox_external_node_versions v ON v.id=n.current_version_id WHERE n.source_id=$1 AND n.identity_epoch=$2")
        .bind(source_id).bind(identity_epoch).fetch_all(&mut *connection).await?;
    let mut previous: BTreeMap<_, _> = previous
        .into_iter()
        .map(|value| (value.identity_key.clone(), value))
        .collect();
    let rejected: Value = serde_json::to_value(&batch.rejected).map_err(anyhow::Error::from)?;
    let unsupported = batch.rejected.len();
    let revision: i64 = sqlx::query_scalar("INSERT INTO singbox_source_revisions(source_id,settings_revision,identity_epoch,parser_version,body_sha256,format,supported_count,unsupported_count,rejected_nodes,fetched_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id")
        .bind(source_id).bind(settings_revision).bind(identity_epoch).bind(PARSER_VERSION).bind(body_sha256).bind(batch.format).bind(batch.nodes.len() as i32).bind(unsupported as i32).bind(rejected).bind(now).fetch_one(&mut *connection).await?;
    sqlx::query("UPDATE singbox_external_nodes SET present=FALSE,identity_unique=TRUE WHERE source_id=$1 AND identity_epoch=$2")
        .bind(source_id).bind(identity_epoch).execute(&mut *connection).await?;
    for key in batch.ambiguous_keys {
        previous.remove(&key);
        sqlx::query("UPDATE singbox_external_nodes SET present=TRUE,identity_unique=FALSE,last_seen_revision_id=$4 WHERE source_id=$1 AND identity_epoch=$2 AND identity_key=$3")
            .bind(source_id).bind(identity_epoch).bind(key).bind(revision).execute(&mut *connection).await?;
    }
    let mut added = 0;
    let mut updated = 0;
    for node in batch.nodes {
        let config = serde_json::to_value(&node.outbound).map_err(anyhow::Error::from)?;
        let hash = parse::digest(&serde_json::to_vec(&config).map_err(anyhow::Error::from)?);
        match previous.remove(&node.identity_key) {
            None => added += 1,
            Some(old)
                if !old.present
                    || old.name != node.name
                    || old.config_sha256.as_deref() != Some(&hash) =>
            {
                updated += 1
            }
            Some(_) => {}
        }
        let node_id: i64 = sqlx::query_scalar("INSERT INTO singbox_external_nodes(source_id,identity_epoch,identity_key,name,present,identity_unique,last_seen_revision_id) VALUES($1,$2,$3,$4,TRUE,TRUE,$5) ON CONFLICT(source_id,identity_epoch,identity_key) DO UPDATE SET name=EXCLUDED.name,present=TRUE,identity_unique=TRUE,last_seen_revision_id=EXCLUDED.last_seen_revision_id RETURNING id")
            .bind(source_id).bind(identity_epoch).bind(node.identity_key).bind(&node.name).bind(revision).fetch_one(&mut *connection).await?;
        let capabilities =
            serde_json::to_value(node.outbound.capabilities().map_err(anyhow::Error::from)?)
                .map_err(anyhow::Error::from)?;
        let version: i64 = sqlx::query_scalar("INSERT INTO singbox_external_node_versions(external_node_id,source_id,source_revision_id,identity_epoch,parser_version,name,config_json,config_sha256,capabilities_json,created_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING id")
            .bind(node_id).bind(source_id).bind(revision).bind(identity_epoch).bind(PARSER_VERSION).bind(node.name).bind(config).bind(hash).bind(capabilities).bind(now).fetch_one(&mut *connection).await?;
        sqlx::query("UPDATE singbox_external_nodes SET current_version_id=$2 WHERE id=$1")
            .bind(node_id)
            .bind(version)
            .execute(&mut *connection)
            .await?;
    }
    let missing = previous.values().filter(|value| value.present).count();
    let changes =
        json!({"added":added,"updated":updated,"missing":missing,"unsupported":unsupported});
    sqlx::query("UPDATE singbox_subscription_sources SET changes=$2 WHERE id=$1")
        .bind(source_id)
        .bind(changes)
        .execute(&mut *connection)
        .await?;
    Ok(revision)
}

pub(super) async fn save_traffic_on(
    connection: &mut PgConnection,
    source_id: i64,
    traffic: Option<Value>,
    now: i64,
) -> ApiResult<()> {
    if let Some(mut traffic) = traffic {
        traffic["updated_at"] = now.into();
        sqlx::query("UPDATE singbox_subscription_sources SET traffic=$2 WHERE id=$1")
            .bind(source_id)
            .bind(traffic)
            .execute(&mut *connection)
            .await?;
    }
    Ok(())
}
