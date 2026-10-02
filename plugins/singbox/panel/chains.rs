use crate::{
    AppState,
    auth::require_admin,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sinan_compiler::Relay;
use sqlx::{FromRow, Postgres, Transaction};
use uuid::Uuid;

#[derive(Serialize, FromRow)]
pub struct Chain {
    pub id: i64,
    pub name: String,
    pub entry_node_id: i64,
    pub exit_node_id: i64,
    pub available: bool,
}
const SELECT: &str = "SELECT c.id,c.name,c.entry_node_id,c.exit_node_id,(n.enabled AND e.enabled AND n.deleted_at IS NULL AND e.deleted_at IS NULL AND n.protocol='vless-reality' AND e.protocol='vless-reality' AND ns.deleted_at IS NULL AND es.deleted_at IS NULL) AS available FROM singbox_live_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id JOIN servers ns ON ns.id=n.server_id JOIN servers es ON es.id=e.server_id";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChainRequest {
    pub name: String,
    pub entry_node_id: i64,
    pub exit_node_id: i64,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Chain>>> {
    require_admin(&state, &headers).await?;
    Ok(Json(
        sqlx::query_as(&format!("{SELECT} ORDER BY c.id"))
            .fetch_all(&state.pool)
            .await?,
    ))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ChainRequest>,
) -> ApiResult<(StatusCode, Json<Chain>)> {
    require_admin(&state, &headers).await?;
    let name = super::business::name(&request.name)?;
    if request.entry_node_id <= 0
        || request.exit_node_id <= 0
        || request.entry_node_id == request.exit_node_id
    {
        return Err(ApiError::BadRequest(
            "请选择不同服务器上的入口节点和出口节点".into(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    super::entitlements::lock(&mut tx).await?;
    let servers: Vec<i64> = sqlx::query_scalar("SELECT DISTINCT n.server_id FROM nodes n JOIN servers s ON s.id=n.server_id WHERE n.id=ANY($1) AND n.deleted_at IS NULL AND s.deleted_at IS NULL AND n.protocol='vless-reality' ORDER BY n.server_id")
        .bind(vec![request.entry_node_id, request.exit_node_id]).fetch_all(&mut *tx).await?;
    if servers.len() != 2 {
        return Err(ApiError::BadRequest(
            "两跳仅支持 VLESS + Reality 节点；入口和出口必须属于两台不同服务器，且均未删除".into(),
        ));
    }
    for server in &servers {
        super::business::lock_server(&mut tx, *server).await?;
    }
    // A dedicated entry prevents silently turning an existing direct grant into a chain.
    // Exits may be shared, but cannot themselves be chain entries (no cycles/nesting).
    let conflict: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM singbox_live_chains WHERE entry_node_id=ANY($1) OR exit_node_id=$2) OR EXISTS(SELECT 1 FROM accesses WHERE node_id=$2) OR EXISTS(SELECT 1 FROM singbox_policy_nodes WHERE node_id=$2) OR EXISTS(SELECT 1 FROM singbox_chain_hops h JOIN singbox_live_chains c ON c.id=h.chain_id WHERE h.managed_node_id=$2)")
        .bind(vec![request.entry_node_id, request.exit_node_id]).bind(request.entry_node_id).fetch_one(&mut *tx).await?;
    if conflict {
        return Err(ApiError::Conflict(
            "入口需要使用尚未授权的独立节点；不支持重复入口、嵌套或循环链路".into(),
        ));
    }
    let id: i64 = sqlx::query_scalar("INSERT INTO singbox_chains(name,entry_node_id,exit_node_id,relay_uuid) VALUES($1,$2,$3,$4) RETURNING id")
        .bind(name).bind(request.entry_node_id).bind(request.exit_node_id).bind(Uuid::new_v4()).fetch_one(&mut *tx).await?;
    super::mixed_paths::seed_legacy_on(&mut tx, id).await?;
    super::business::mark_dirty(&mut tx, &servers).await?;
    let value = sqlx::query_as(&format!("{SELECT} WHERE c.id=$1"))
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(value)))
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    super::entitlements::lock(&mut tx).await?;
    let used: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM singbox_policy_chains WHERE chain_id=$1)")
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;
    if used {
        return Err(ApiError::Conflict("请先从策略组移除此链路".into()));
    }
    let servers: Vec<i64> = sqlx::query_scalar("SELECT unnest(ARRAY[n.server_id,e.server_id]) FROM singbox_live_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id WHERE c.id=$1")
        .bind(id).fetch_all(&mut *tx).await?;
    super::business::mark_dirty(&mut tx, &servers).await?;
    if sqlx::query("UPDATE singbox_chains SET deleted_at=FLOOR(EXTRACT(EPOCH FROM clock_timestamp()))::bigint WHERE id=$1 AND deleted_at IS NULL AND path_kind='legacy'")
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        == 0
    {
        return Err(ApiError::NotFound);
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn ensure_direct(tx: &mut Transaction<'_, Postgres>, node: i64) -> ApiResult<()> {
    let entry: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM singbox_live_chains WHERE entry_node_id=$1)",
    )
    .bind(node)
    .fetch_one(&mut **tx)
    .await?;
    if entry {
        return Err(ApiError::Conflict(
            "此节点是链路入口，请通过包含该链路的策略组授权".into(),
        ));
    }
    Ok(())
}

#[derive(FromRow)]
struct RelayRow {
    exit_settings: serde_json::Value,
    chain_id: i64,
    entry_node_id: i64,
    exit_node_id: i64,
    uuid: Uuid,
    entry_protocol: String,
    exit_protocol: String,
    public_host: String,
    port: i32,
    sni: String,
    public_key: String,
    short_id: String,
}

pub(crate) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    server: i64,
    at: i64,
) -> anyhow::Result<Vec<Relay>> {
    let rows = sqlx::query_as::<_, RelayRow>("SELECT c.id AS chain_id,c.entry_node_id,c.exit_node_id,c.relay_uuid AS uuid,n.protocol AS entry_protocol,e.protocol AS exit_protocol,e.public_host,e.port,e.sni,e.public_key,e.short_id,e.settings AS exit_settings FROM singbox_live_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id JOIN servers ns ON ns.id=n.server_id JOIN servers es ON es.id=e.server_id WHERE (n.server_id=$1 OR e.server_id=$1) AND n.enabled AND e.enabled AND n.deleted_at IS NULL AND e.deleted_at IS NULL AND n.protocol='vless-reality' AND e.protocol='vless-reality' AND ns.deleted_at IS NULL AND es.deleted_at IS NULL AND EXISTS(SELECT 1 FROM singbox_eligible_accesses($2) a WHERE a.node_id=n.id) ORDER BY c.id")
        .bind(server).bind(at).fetch_all(&mut **tx).await?;
    rows.into_iter()
        .map(|r| {
            let settings: sinan_compiler::NodeSettings = serde_json::from_value(r.exit_settings)?;
            anyhow::ensure!(
                r.entry_protocol == "vless-reality" && r.exit_protocol == "vless-reality",
                "two-hop relays require VLESS + Reality at both ends"
            );
            Ok(Relay {
                fingerprint: settings.reality.fingerprint,
                settings: settings.clone(),
                chain_id: r.chain_id,
                entry_node_id: r.entry_node_id,
                exit_node_id: r.exit_node_id,
                uuid: r.uuid,
                public_host: r.public_host,
                port: settings.public_port.unwrap_or(r.port.try_into()?),
                sni: r.sni,
                public_key: r.public_key,
                short_id: r.short_id,
            })
        })
        .collect()
}
