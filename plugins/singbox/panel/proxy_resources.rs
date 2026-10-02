use super::business;
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
use serde::Serialize;
use sinan_protocol::now_timestamp;
use sqlx::{FromRow, Postgres, Transaction};
use std::collections::BTreeMap;

#[derive(Clone, Serialize, FromRow)]
pub struct ResourceEndpoint {
    pub id: i64,
    pub name: String,
    pub server_id: i64,
    pub server_name: String,
    pub protocol: String,
    pub port: i32,
    #[sqlx(skip)]
    pub public_port: i32,
    pub public_host: String,
    pub sni: String,
    pub enabled: bool,
    pub node_deleted: bool,
    pub server_deleted: bool,
    pub plugin_enabled: bool,
    pub online: bool,
    pub desired_revision: Option<i64>,
    pub applied_revision: Option<i64>,
    pub applied_observed_at: Option<i64>,
    #[serde(skip)]
    settings: serde_json::Value,
    #[serde(skip)]
    protocol_config: serde_json::Value,
    #[serde(skip)]
    #[sqlx(skip)]
    public_port_valid: bool,
    #[serde(skip)]
    #[sqlx(skip)]
    protocol_config_valid: bool,
}

#[derive(Serialize)]
pub struct ChainReference {
    pub id: i64,
    pub name: String,
    pub role: &'static str,
}

#[derive(Serialize)]
pub struct ProxyResource {
    pub kind: &'static str,
    pub id: i64,
    pub name: String,
    pub entry: ResourceEndpoint,
    pub exit: Option<ResourceEndpoint>,
    /// Structural eligibility only; application and connectivity are separate.
    pub available: bool,
    pub unavailable_reasons: Vec<String>,
    pub policy_group_ids: Vec<i64>,
    pub user_count: i64,
    pub chain_refs: Vec<ChainReference>,
}

#[derive(FromRow)]
struct ChainRow {
    id: i64,
    name: String,
    entry_node_id: i64,
    exit_node_id: i64,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ProxyResource>>> {
    require_admin(&state, &headers).await?;
    Ok(Json(read_resources(&state).await?))
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, i64)>,
) -> ApiResult<Json<ProxyResource>> {
    require_admin(&state, &headers).await?;
    validate_identity(&kind, id)?;
    let resource = read_resources(&state)
        .await?
        .into_iter()
        .find(|resource| resource.kind == kind && resource.id == id)
        .ok_or(ApiError::NotFound)?;
    Ok(Json(resource))
}

fn validate_identity(kind: &str, id: i64) -> ApiResult<()> {
    if !matches!(kind, "direct" | "chain") || id <= 0 {
        return Err(ApiError::BadRequest("请选择有效的直连或链路资源".into()));
    }
    Ok(())
}

async fn read_resources(state: &AppState) -> ApiResult<Vec<ProxyResource>> {
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let query = format!(
        "SELECT n.id,n.name,n.server_id,s.name AS server_name,n.protocol,n.port,n.settings,n.protocol_config,n.public_host,n.sni,n.enabled,n.deleted_at IS NOT NULL AS node_deleted,s.deleted_at IS NOT NULL AS server_deleted,({}) IS NOT NULL AS plugin_enabled,(s.last_seen IS NOT NULL AND $1-s.last_seen<=60) AS online,m.target_rev AS desired_revision,m.applied_rev AS applied_revision,m.updated_at AS applied_observed_at FROM nodes n JOIN servers s ON s.id=n.server_id LEFT JOIN server_plugins p ON p.server_id=s.id AND p.plugin='sing-box' LEFT JOIN server_module_status m ON m.server_id=s.id AND m.module='singbox' WHERE (n.deleted_at IS NULL AND s.deleted_at IS NULL) OR EXISTS(SELECT 1 FROM singbox_chains c WHERE c.entry_node_id=n.id OR c.exit_node_id=n.id) ORDER BY n.id",
        super::settings::SOURCE_SQL
    );
    let endpoints: BTreeMap<i64, ResourceEndpoint> = sqlx::query_as::<_, ResourceEndpoint>(&query)
        .bind(now_timestamp())
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|mut endpoint| {
            let settings = serde_json::from_value::<sinan_compiler::NodeSettings>(std::mem::take(
                &mut endpoint.settings,
            ));
            let protocol = serde_json::from_value::<sinan_compiler::ProtocolConfig>(
                std::mem::take(&mut endpoint.protocol_config),
            );
            endpoint.protocol_config_valid =
                protocol.is_ok_and(|config| config.kind() == endpoint.protocol);
            endpoint.public_port = endpoint.port;
            if let Ok(settings) = settings
                && settings.public_port != Some(0)
            {
                endpoint.public_port = settings.public_port.map(i32::from).unwrap_or(endpoint.port);
                endpoint.public_port_valid = true;
            }
            (endpoint.id, endpoint)
        })
        .collect();
    let chains = sqlx::query_as::<_, ChainRow>(
        "SELECT id,name,entry_node_id,exit_node_id FROM singbox_chains ORDER BY id",
    )
    .fetch_all(&mut *tx)
    .await?;
    let mut resources = Vec::new();
    for endpoint in endpoints.values() {
        if endpoint.node_deleted
            || endpoint.server_deleted
            || chains
                .iter()
                .any(|chain| chain.entry_node_id == endpoint.id)
        {
            continue;
        }
        let policy_group_ids = sqlx::query_scalar(
            "SELECT group_id FROM singbox_policy_nodes WHERE node_id=$1 ORDER BY group_id",
        )
        .bind(endpoint.id)
        .fetch_all(&mut *tx)
        .await?;
        let user_count = sqlx::query_scalar("SELECT COUNT(DISTINCT g.user_id) FROM (SELECT user_id FROM accesses WHERE node_id=$1 AND direct_grant UNION SELECT up.user_id FROM singbox_user_policies up JOIN singbox_policy_nodes pn ON pn.group_id=up.group_id WHERE pn.node_id=$1) g JOIN users u ON u.id=g.user_id WHERE u.deleted_at IS NULL")
            .bind(endpoint.id).fetch_one(&mut *tx).await?;
        let reasons = endpoint_reasons(endpoint, "节点");
        resources.push(ProxyResource {
            kind: "direct",
            id: endpoint.id,
            name: endpoint.name.clone(),
            entry: endpoint.clone(),
            exit: None,
            available: reasons.is_empty(),
            unavailable_reasons: reasons,
            policy_group_ids,
            user_count,
            chain_refs: chains
                .iter()
                .filter(|chain| chain.exit_node_id == endpoint.id)
                .map(|chain| ChainReference {
                    id: chain.id,
                    name: chain.name.clone(),
                    role: "exit",
                })
                .collect(),
        });
    }
    for chain in chains {
        // Node/server identities are retained by foreign keys and soft deletion.
        let entry = endpoints
            .get(&chain.entry_node_id)
            .ok_or_else(|| anyhow::anyhow!("chain entry identity is missing"))?;
        let exit = endpoints
            .get(&chain.exit_node_id)
            .ok_or_else(|| anyhow::anyhow!("chain exit identity is missing"))?;
        let mut reasons = endpoint_reasons(entry, "入口");
        reasons.extend(endpoint_reasons(exit, "出口"));
        if entry.protocol != "vless-reality" || exit.protocol != "vless-reality" {
            reasons.push("现有受管两跳需要两端均使用 VLESS + Reality".into());
        }
        if entry.server_id == exit.server_id {
            reasons.push("入口与出口必须属于不同服务器".into());
        }
        let policy_group_ids = sqlx::query_scalar(
            "SELECT group_id FROM singbox_policy_chains WHERE chain_id=$1 ORDER BY group_id",
        )
        .bind(chain.id)
        .fetch_all(&mut *tx)
        .await?;
        let user_count = sqlx::query_scalar("SELECT COUNT(DISTINCT up.user_id) FROM singbox_user_policies up JOIN singbox_policy_chains pc ON pc.group_id=up.group_id JOIN users u ON u.id=up.user_id WHERE pc.chain_id=$1 AND u.deleted_at IS NULL")
            .bind(chain.id).fetch_one(&mut *tx).await?;
        resources.push(ProxyResource {
            kind: "chain",
            id: chain.id,
            name: chain.name,
            entry: entry.clone(),
            exit: Some(exit.clone()),
            available: reasons.is_empty(),
            unavailable_reasons: reasons,
            policy_group_ids,
            user_count,
            chain_refs: vec![],
        });
    }
    tx.commit().await?;
    Ok(resources)
}

fn endpoint_reasons(endpoint: &ResourceEndpoint, role: &str) -> Vec<String> {
    let mut reasons = Vec::new();
    if endpoint.node_deleted {
        reasons.push(format!("{role}节点已删除"));
    }
    if endpoint.server_deleted {
        reasons.push(format!("{role}服务器已退役"));
    }
    if !endpoint.enabled {
        reasons.push(format!("{role}节点已停用"));
    }
    if !endpoint.plugin_enabled {
        reasons.push(format!("{role}服务器未启用 sing-box 插件"));
    }
    if !endpoint.public_port_valid {
        reasons.push(format!("{role}公开端口参数无法确认，暂显示监听端口"));
    }
    if !endpoint.protocol_config_valid {
        reasons.push(format!("{role}协议参数无法解析或与协议类型不一致"));
    }
    reasons
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((kind, id)): Path<(String, i64)>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    validate_identity(&kind, id)?;
    if kind == "direct" {
        remove_direct_node(&state, id).await?;
    } else {
        remove_chain_resource(&state, id).await?;
    }
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn remove_direct_node(state: &AppState, id: i64) -> ApiResult<()> {
    let mut tx = state.pool.begin().await?;
    super::entitlements::lock(&mut tx).await?;
    let server: i64 =
        sqlx::query_scalar("SELECT server_id FROM nodes WHERE id=$1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or(ApiError::NotFound)?;
    lock_cleanup_servers(&mut tx, &[server]).await?;
    ensure_node_unreferenced(&mut tx, id).await?;
    soft_delete_node(&mut tx, id).await?;
    business::mark_dirty(&mut tx, &[server]).await?;
    tx.commit().await?;
    Ok(())
}

async fn remove_chain_resource(state: &AppState, id: i64) -> ApiResult<()> {
    let mut tx = state.pool.begin().await?;
    super::entitlements::lock(&mut tx).await?;
    let chain = sqlx::query_as::<_, ChainRow>(
        "SELECT id,name,entry_node_id,exit_node_id FROM singbox_chains WHERE id=$1",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    let servers: Vec<i64> = sqlx::query_scalar(
        "SELECT DISTINCT server_id FROM nodes WHERE id=ANY($1) ORDER BY server_id",
    )
    .bind(vec![chain.entry_node_id, chain.exit_node_id])
    .fetch_all(&mut *tx)
    .await?;
    lock_cleanup_servers(&mut tx, &servers).await?;
    ensure_chain_unreferenced(&mut tx, id).await?;
    // Protect even legacy/corrupt direct references to this dedicated entry.
    let policies = node_policies(&mut tx, chain.entry_node_id).await?;
    if !policies.is_empty() {
        return Err(reference_error(policies, vec![]));
    }
    sqlx::query("DELETE FROM singbox_chains WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    ensure_node_unreferenced(&mut tx, chain.entry_node_id).await?;
    soft_delete_node(&mut tx, chain.entry_node_id).await?;
    business::mark_dirty(&mut tx, &servers).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn lock_cleanup_servers(
    tx: &mut Transaction<'_, Postgres>,
    servers: &[i64],
) -> ApiResult<()> {
    // Retired server identities still protect shared resources during cleanup.
    sqlx::query("SELECT id FROM servers WHERE id=ANY($1) ORDER BY id FOR UPDATE")
        .bind(servers)
        .fetch_all(&mut **tx)
        .await?;
    Ok(())
}

async fn soft_delete_node(tx: &mut Transaction<'_, Postgres>, id: i64) -> ApiResult<()> {
    sqlx::query("UPDATE nodes SET deleted_at=COALESCE(deleted_at,$2) WHERE id=$1")
        .bind(id)
        .bind(now_timestamp())
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM accesses WHERE node_id=$1")
        .bind(id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[derive(Serialize, FromRow)]
struct PolicyReference {
    id: i64,
    name: String,
}

#[derive(Serialize, FromRow)]
struct NodeChainReference {
    id: i64,
    name: String,
    role: String,
}

async fn node_policies(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
) -> ApiResult<Vec<PolicyReference>> {
    Ok(sqlx::query_as("SELECT p.id,p.name FROM singbox_policy_groups p JOIN singbox_policy_nodes n ON n.group_id=p.id WHERE n.node_id=$1 ORDER BY p.id")
        .bind(id).fetch_all(&mut **tx).await?)
}

pub(super) async fn ensure_chain_unreferenced(
    tx: &mut Transaction<'_, Postgres>,
    id: i64,
) -> ApiResult<()> {
    let policies = sqlx::query_as::<_, PolicyReference>("SELECT p.id,p.name FROM singbox_policy_groups p JOIN singbox_policy_chains c ON c.group_id=p.id WHERE c.chain_id=$1 ORDER BY p.id")
        .bind(id).fetch_all(&mut **tx).await?;
    if !policies.is_empty() {
        return Err(reference_error(policies, vec![]));
    }
    Ok(())
}

async fn ensure_node_unreferenced(tx: &mut Transaction<'_, Postgres>, id: i64) -> ApiResult<()> {
    let policies = node_policies(tx, id).await?;
    let chains = sqlx::query_as::<_, NodeChainReference>("SELECT id,name,CASE WHEN entry_node_id=$1 THEN 'entry' ELSE 'exit' END AS role FROM singbox_chains WHERE entry_node_id=$1 OR exit_node_id=$1 ORDER BY id")
        .bind(id).fetch_all(&mut **tx).await?;
    if !policies.is_empty() || !chains.is_empty() {
        return Err(reference_error(policies, chains));
    }
    Ok(())
}

fn reference_error(policies: Vec<PolicyReference>, chains: Vec<NodeChainReference>) -> ApiError {
    let mut names: Vec<String> = policies
        .iter()
        .map(|reference| format!("策略组 #{}「{}」", reference.id, reference.name))
        .collect();
    names.extend(chains.iter().map(|reference| {
        let role = if reference.role == "entry" {
            "入口"
        } else {
            "出口"
        };
        format!("链路 #{}「{}」({role})", reference.id, reference.name)
    }));
    ApiError::ConflictReferences {
        message: format!("资源仍被引用，请先解除：{}", names.join("、")),
        references: serde_json::json!({"policies":policies,"chains":chains}),
    }
}
