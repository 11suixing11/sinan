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
use sha2::{Digest, Sha256};
use sinan_compiler::Relay;
use sqlx::{FromRow, Postgres, Transaction};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchRequest {
    pub request_id: Uuid,
    pub items: Vec<BatchItem>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatchItem {
    pub name: String,
    pub entry: BatchEntry,
    pub hops: Vec<BatchHop>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum BatchEntry {
    New {
        server_id: i64,
        public_host: String,
        sni: String,
        port: Option<i64>,
    },
    Existing {
        node_id: i64,
    },
}

#[derive(Deserialize, Serialize)]
pub struct BatchHop {
    pub kind: String,
    #[serde(default)]
    pub node_id: Option<i64>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatchReceipt {
    pub request_id: Uuid,
    pub chain_ids: Vec<i64>,
    pub entry_node_ids: Vec<i64>,
}

pub async fn create_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<BatchRequest>,
) -> ApiResult<(StatusCode, Json<BatchReceipt>)> {
    require_admin(&state, &headers).await?;
    if request.request_id.is_nil() || request.items.is_empty() || request.items.len() > 32 {
        return Err(ApiError::BadRequest(
            "一次创建需要有效请求 ID 和 1 至 32 条链路".into(),
        ));
    }
    for (index, item) in request.items.iter_mut().enumerate() {
        item.name = super::business::name(&item.name).map_err(|error| item_error(index, error))?;
        if item.hops.len() != 1 || item.hops[0].kind != "managed" {
            return Err(ApiError::BadRequest(format!(
                "第 {} 条：本次仅支持一个受管出口；混合订阅及多段路径尚未实现",
                index + 1
            )));
        }
        if item.hops[0].node_id.is_none_or(|id| id <= 0) || !item.hops[0].additional.is_empty() {
            return Err(ApiError::BadRequest(format!(
                "第 {} 条：请选择有效出口节点",
                index + 1
            )));
        }
        match &item.entry {
            BatchEntry::New {
                server_id, port, ..
            } => {
                if *server_id <= 0 {
                    return Err(ApiError::BadRequest(format!(
                        "第 {} 条：请选择有效入口服务器",
                        index + 1
                    )));
                }
                if let Some(port) = port {
                    super::nodes::validate_port(*port).map_err(|error| item_error(index, error))?;
                }
            }
            BatchEntry::Existing { node_id } if *node_id <= 0 => {
                return Err(ApiError::BadRequest(format!(
                    "第 {} 条：请选择有效入口节点",
                    index + 1
                )));
            }
            BatchEntry::Existing { .. } => {}
        }
    }
    let normalized = serde_json::to_vec(&request.items).map_err(anyhow::Error::from)?;
    let request_sha256 = format!("{:x}", Sha256::digest(&normalized));
    let mut tx = state.pool.begin().await?;
    super::entitlements::lock(&mut tx).await?;
    if let Some((previous_hash, receipt)) = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT request_sha256,receipt FROM singbox_chain_creation_requests WHERE request_id=$1",
    )
    .bind(request.request_id)
    .fetch_optional(&mut *tx)
    .await?
    {
        if previous_hash != request_sha256 {
            return Err(ApiError::Conflict(
                "此请求 ID 已用于不同内容，请为修改后的草稿使用新的请求 ID".into(),
            ));
        }
        let receipt: BatchReceipt = serde_json::from_value(receipt).map_err(anyhow::Error::from)?;
        tx.commit().await?;
        // A replay reports original IDs even after resource deletion, never recreates.
        return Ok((StatusCode::OK, Json(receipt)));
    }
    let mut servers = BTreeSet::new();
    for (index, item) in request.items.iter().enumerate() {
        let mut node_ids = vec![item.hops[0].node_id.expect("validated managed hop")];
        match &item.entry {
            BatchEntry::New { server_id, .. } => {
                servers.insert(*server_id);
            }
            BatchEntry::Existing { node_id } => node_ids.push(*node_id),
        }
        for node_id in node_ids {
            let server = sqlx::query_scalar::<_, i64>(
                "SELECT server_id FROM nodes WHERE id=$1 AND deleted_at IS NULL",
            )
            .bind(node_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(ApiError::NotFound)
            .map_err(|error| item_error(index, error))?;
            servers.insert(server);
        }
    }
    let servers: Vec<i64> = servers.into_iter().collect();
    for server in &servers {
        let live: bool = sqlx::query_scalar("SELECT deleted_at IS NULL AND NOT EXISTS(SELECT 1 FROM server_retirements r WHERE r.server_id=s.id AND r.status IN ('pending','failed','offline_unconfirmed')) FROM servers s WHERE id=$1 FOR UPDATE")
            .bind(server).fetch_optional(&mut *tx).await?
            .ok_or_else(|| ApiError::BadRequest(format!("服务器 #{server} 不存在")))?;
        if !live {
            return Err(ApiError::Conflict(format!(
                "服务器 #{server} 已退役或正在退役，不能创建链路"
            )));
        }
        super::settings::require_enabled(&mut tx, *server).await?;
    }
    let mut receipt = BatchReceipt {
        request_id: request.request_id,
        chain_ids: Vec::with_capacity(request.items.len()),
        entry_node_ids: Vec::with_capacity(request.items.len()),
    };
    for (index, item) in request.items.into_iter().enumerate() {
        let entry_node_id = match item.entry {
            BatchEntry::Existing { node_id } => node_id,
            BatchEntry::New {
                server_id,
                public_host,
                sni,
                port,
            } => {
                super::nodes::create_locked(
                    &mut tx,
                    super::nodes::CreateNode {
                        enabled: Some(true),
                        settings: Default::default(),
                        name: item.name.clone(),
                        server_id,
                        public_host,
                        sni,
                        protocol_config: Default::default(),
                        port,
                    },
                )
                .await
                .map_err(|error| item_error(index, error))?
                .id
            }
        };
        let chain_id = create_locked(
            &mut tx,
            &item.name,
            entry_node_id,
            item.hops[0].node_id.expect("validated managed hop"),
        )
        .await
        .map_err(|error| item_error(index, error))?;
        receipt.chain_ids.push(chain_id);
        receipt.entry_node_ids.push(entry_node_id);
    }
    super::business::mark_dirty(&mut tx, &servers).await?;
    sqlx::query("INSERT INTO singbox_chain_creation_requests(request_id,request_sha256,receipt,created_at) VALUES($1,$2,$3,$4)")
        .bind(receipt.request_id).bind(request_sha256)
        .bind(serde_json::to_value(&receipt).map_err(anyhow::Error::from)?)
        .bind(sinan_protocol::now_timestamp()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(receipt)))
}

fn item_error(index: usize, error: ApiError) -> ApiError {
    match error {
        ApiError::BadRequest(message) => {
            ApiError::BadRequest(format!("第 {} 条：{message}", index + 1))
        }
        ApiError::Conflict(message) => {
            ApiError::Conflict(format!("第 {} 条：{message}", index + 1))
        }
        ApiError::NotFound => {
            ApiError::BadRequest(format!("第 {} 条：节点不存在或已删除", index + 1))
        }
        error => error,
    }
}

async fn create_locked(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    entry_id: i64,
    exit_id: i64,
) -> ApiResult<i64> {
    if entry_id == exit_id {
        return Err(ApiError::BadRequest("入口与出口不能是同一节点".into()));
    }
    let query = format!(
        "SELECT {} FROM nodes n WHERE n.id=ANY($1) AND n.deleted_at IS NULL ORDER BY n.id FOR UPDATE",
        super::business::NODE_COLUMNS
    );
    let nodes = sqlx::query_as::<_, super::business::NodeRow>(&query)
        .bind(vec![entry_id, exit_id])
        .fetch_all(&mut **tx)
        .await?;
    let entry = nodes
        .iter()
        .find(|node| node.id == entry_id)
        .ok_or(ApiError::NotFound)?;
    let exit = nodes
        .iter()
        .find(|node| node.id == exit_id)
        .ok_or(ApiError::NotFound)?;
    if entry.server_id == exit.server_id
        || !entry.enabled
        || !exit.enabled
        || entry.protocol != "vless-reality"
        || exit.protocol != "vless-reality"
    {
        return Err(ApiError::BadRequest(
            "入口和出口必须是不同服务器上的已启用 VLESS + Reality 节点".into(),
        ));
    }
    let conflict: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM singbox_chains WHERE entry_node_id=ANY($1) OR exit_node_id=$2) OR EXISTS(SELECT 1 FROM accesses WHERE node_id=$2) OR EXISTS(SELECT 1 FROM singbox_policy_nodes WHERE node_id=$2)")
        .bind(vec![entry_id, exit_id]).bind(entry_id).fetch_one(&mut **tx).await?;
    if conflict {
        return Err(ApiError::Conflict(
            "入口需要尚未授权的独立节点；不支持重复入口、嵌套或循环链路".into(),
        ));
    }
    for node in [entry, exit] {
        let model = node.model(vec![]).map_err(|_| {
            ApiError::BadRequest(format!(
                "节点 #{} 的协议或参数无法解析，请先修复节点配置",
                node.id
            ))
        })?;
        if !model.protocol_config.is_reality() {
            return Err(ApiError::BadRequest(format!(
                "节点 #{} 的实际协议配置不是 VLESS + Reality",
                node.id
            )));
        }
        super::business::validate_node(node)?;
        super::nodes::validate_server_config(tx, node).await?;
    }
    Ok(sqlx::query_scalar("INSERT INTO singbox_chains(name,entry_node_id,exit_node_id,relay_uuid) VALUES($1,$2,$3,$4) RETURNING id")
        .bind(name).bind(entry_id).bind(exit_id).bind(Uuid::new_v4()).fetch_one(&mut **tx).await?)
}

#[derive(Serialize, FromRow)]
pub struct Chain {
    pub id: i64,
    pub name: String,
    pub entry_node_id: i64,
    pub exit_node_id: i64,
    pub available: bool,
}
const SELECT: &str = "SELECT c.id,c.name,c.entry_node_id,c.exit_node_id,(n.enabled AND e.enabled AND n.deleted_at IS NULL AND e.deleted_at IS NULL AND n.protocol='vless-reality' AND e.protocol='vless-reality' AND ns.deleted_at IS NULL AND es.deleted_at IS NULL) AS available FROM singbox_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id JOIN servers ns ON ns.id=n.server_id JOIN servers es ON es.id=e.server_id";

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
    let conflict: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM singbox_chains WHERE entry_node_id=ANY($1) OR exit_node_id=$2) OR EXISTS(SELECT 1 FROM accesses WHERE node_id=$2) OR EXISTS(SELECT 1 FROM singbox_policy_nodes WHERE node_id=$2)")
        .bind(vec![request.entry_node_id, request.exit_node_id]).bind(request.entry_node_id).fetch_one(&mut *tx).await?;
    if conflict {
        return Err(ApiError::Conflict(
            "入口需要使用尚未授权的独立节点；不支持重复入口、嵌套或循环链路".into(),
        ));
    }
    let id: i64 = sqlx::query_scalar("INSERT INTO singbox_chains(name,entry_node_id,exit_node_id,relay_uuid) VALUES($1,$2,$3,$4) RETURNING id")
        .bind(name).bind(request.entry_node_id).bind(request.exit_node_id).bind(Uuid::new_v4()).fetch_one(&mut *tx).await?;
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
    let servers: Vec<i64> = sqlx::query_scalar("SELECT unnest(ARRAY[n.server_id,e.server_id]) FROM singbox_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id WHERE c.id=$1")
        .bind(id).fetch_all(&mut *tx).await?;
    super::proxy_resources::lock_cleanup_servers(&mut tx, &servers).await?;
    super::proxy_resources::ensure_chain_unreferenced(&mut tx, id).await?;
    super::business::mark_dirty(&mut tx, &servers).await?;
    if sqlx::query("DELETE FROM singbox_chains WHERE id=$1")
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
    let entry: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM singbox_chains WHERE entry_node_id=$1)")
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
    let rows = sqlx::query_as::<_, RelayRow>("SELECT c.id AS chain_id,c.entry_node_id,c.exit_node_id,c.relay_uuid AS uuid,n.protocol AS entry_protocol,e.protocol AS exit_protocol,e.public_host,e.port,e.sni,e.public_key,e.short_id,e.settings AS exit_settings FROM singbox_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id JOIN servers ns ON ns.id=n.server_id JOIN servers es ON es.id=e.server_id WHERE (n.server_id=$1 OR e.server_id=$1) AND n.enabled AND e.enabled AND n.deleted_at IS NULL AND e.deleted_at IS NULL AND n.protocol='vless-reality' AND e.protocol='vless-reality' AND ns.deleted_at IS NULL AND es.deleted_at IS NULL AND EXISTS(SELECT 1 FROM singbox_eligible_accesses($2) a WHERE a.node_id=n.id) ORDER BY c.id")
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
