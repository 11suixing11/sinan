use super::{node_protocol::ProtocolInput, node_settings::SettingsInput};
use crate::{
    AppState,
    auth::require_admin,
    error::{ApiError, ApiResult},
    plugins::singbox::business::{self, NODE_COLUMNS, NodeRow, NodeView},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use sinan_protocol::now_timestamp;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNode {
    pub enabled: Option<bool>,
    #[serde(default)]
    pub settings: SettingsInput,
    pub name: String,
    pub server_id: i64,
    pub public_host: String,
    #[serde(default)]
    pub sni: String,
    #[serde(default)]
    pub protocol_config: ProtocolInput,
    pub port: Option<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateNode {
    pub enabled: Option<bool>,
    pub settings: Option<SettingsInput>,
    pub name: Option<String>,
    pub public_host: Option<String>,
    pub sni: Option<String>,
    pub protocol_config: Option<ProtocolInput>,
    pub port: Option<i64>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<NodeView>>> {
    require_admin(&state, &headers).await?;
    let query = format!(
        "SELECT {NODE_COLUMNS} FROM nodes n JOIN servers s ON s.id=n.server_id WHERE n.deleted_at IS NULL AND s.deleted_at IS NULL ORDER BY n.id"
    );
    let rows = sqlx::query_as::<_, NodeRow>(&query)
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(
        rows.into_iter()
            .map(NodeRow::view)
            .collect::<ApiResult<_>>()?,
    ))
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<NodeView>> {
    require_admin(&state, &headers).await?;
    let query = format!(
        "SELECT {NODE_COLUMNS} FROM nodes n JOIN servers s ON s.id=n.server_id WHERE n.id=$1 AND n.deleted_at IS NULL AND s.deleted_at IS NULL"
    );
    let node = sqlx::query_as::<_, NodeRow>(&query)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(node.view()?))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateNode>,
) -> ApiResult<(StatusCode, Json<NodeView>)> {
    require_admin(&state, &headers).await?;
    let requested_port = request.port.map(validate_port).transpose()?;
    let protocol = request.protocol_config.build(None)?;
    let (private_key, public_key) = if protocol.is_reality() {
        business::generate_reality_keypair()
    } else {
        (String::new(), String::new())
    };
    let mut node = NodeRow {
        enabled: request.enabled.unwrap_or(true),
        settings: request.settings.build(&serde_json::json!({}))?,
        id: i64::MAX,
        name: business::name(&request.name)?,
        server_id: request.server_id,
        protocol: protocol.kind().into(),
        protocol_config: serde_json::to_value(&protocol).map_err(anyhow::Error::from)?,
        port: requested_port.unwrap_or(20000),
        public_host: request.public_host,
        sni: request.sni,
        private_key,
        public_key,
        short_id: if protocol.is_reality() {
            business::short_id()
        } else {
            String::new()
        },
    };
    business::validate_node(&node)?;
    let mut transaction = state.pool.begin().await?;
    super::entitlements::lock(&mut transaction).await?;
    business::lock_server(&mut transaction, request.server_id).await?;
    super::settings::require_enabled(&mut transaction, request.server_id).await?;
    if let Some(port) = requested_port {
        ensure_port_available(&mut transaction, node.server_id, port, None).await?;
    } else {
        node.port = sqlx::query_scalar::<_, i32>("SELECT candidate.port FROM generate_series(20000,29999) AS candidate(port) WHERE NOT EXISTS(SELECT 1 FROM nodes WHERE server_id=$1 AND deleted_at IS NULL AND nodes.port=candidate.port) ORDER BY candidate.port LIMIT 1").bind(node.server_id).fetch_optional(&mut *transaction).await?.ok_or_else(|| ApiError::Conflict("服务器没有可分配端口".into()))?;
    }
    validate_server_config(&mut transaction, &node).await?;
    let query = format!(
        "INSERT INTO nodes AS n (name,server_id,protocol,port,public_host,sni,private_key,public_key,short_id,protocol_config,enabled,settings) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12) RETURNING {NODE_COLUMNS}"
    );
    let node = sqlx::query_as::<_, NodeRow>(&query)
        .bind(node.name)
        .bind(node.server_id)
        .bind(node.protocol)
        .bind(node.port)
        .bind(node.public_host)
        .bind(node.sni)
        .bind(node.private_key)
        .bind(node.public_key)
        .bind(node.short_id)
        .bind(node.protocol_config)
        .bind(node.enabled)
        .bind(node.settings)
        .fetch_one(&mut *transaction)
        .await
        .map_err(port_database_error)?;
    business::mark_dirty(&mut transaction, &[node.server_id]).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(node.view()?)))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<UpdateNode>,
) -> ApiResult<Json<NodeView>> {
    require_admin(&state, &headers).await?;
    if request.name.is_none()
        && request.public_host.is_none()
        && request.sni.is_none()
        && request.port.is_none()
        && request.protocol_config.is_none()
        && request.enabled.is_none()
        && request.settings.is_none()
    {
        return Err(ApiError::BadRequest("至少提供一个修改字段".into()));
    }
    let mut transaction = state.pool.begin().await?;
    super::entitlements::lock(&mut transaction).await?;
    let server_id: i64 =
        sqlx::query_scalar("SELECT server_id FROM nodes WHERE id=$1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(ApiError::NotFound)?;
    business::lock_server(&mut transaction, server_id).await?;
    let query =
        format!("SELECT {NODE_COLUMNS} FROM nodes n WHERE n.id=$1 AND n.deleted_at IS NULL");
    let mut node = sqlx::query_as::<_, NodeRow>(&query)
        .bind(id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or(ApiError::NotFound)?;
    let previous = (
        node.enabled,
        node.settings.clone(),
        node.name.clone(),
        node.public_host.clone(),
        node.sni.clone(),
        node.port,
        node.protocol_config.clone(),
    );
    if let Some(enabled) = request.enabled {
        node.enabled = enabled;
    }
    if let Some(settings) = request.settings {
        node.settings = settings.build(&node.settings)?;
    }
    if let Some(port) = request.port {
        node.port = validate_port(port)?;
        ensure_port_available(&mut transaction, server_id, node.port, Some(id)).await?;
    }
    if let Some(name) = request.name {
        node.name = business::name(&name)?;
    }
    if let Some(host) = request.public_host {
        node.public_host = host;
    }
    if let Some(sni) = request.sni {
        node.sni = sni;
    }
    if let Some(input) = request.protocol_config {
        let previous =
            serde_json::from_value(node.protocol_config.clone()).map_err(anyhow::Error::from)?;
        let next = input.build(Some(&previous))?;
        if matches!(previous.tls(), Some(sinan_compiler::TlsConfig::Acme { .. }))
            && let Some(sinan_compiler::TlsConfig::Acme { .. }) = next.tls()
        {
            // Challenge listeners belong to the service; update shared settings atomically.
            sqlx::query("UPDATE nodes SET protocol_config=jsonb_set(protocol_config,'{tls}',$2) WHERE server_id=$1 AND deleted_at IS NULL AND protocol_config->'tls'->>'mode'='acme'")
                .bind(server_id)
                .bind(serde_json::to_value(next.tls()).map_err(anyhow::Error::from)?)
                .execute(&mut *transaction).await?;
        }
        node.protocol_config = serde_json::to_value(next).map_err(anyhow::Error::from)?;
    }
    business::validate_node(&node)?;
    validate_server_config(&mut transaction, &node).await?;
    let path_references:Vec<i64>=sqlx::query_scalar("SELECT DISTINCT c.id FROM singbox_live_chains c LEFT JOIN singbox_chain_hops h ON h.chain_id=c.id WHERE c.path_kind='mixed' AND (c.entry_node_id=$1 OR h.managed_node_id=$1) ORDER BY c.id").bind(id).fetch_all(&mut *transaction).await?;
    if !path_references.is_empty()
        && (previous.1 != node.settings
            || previous.3 != node.public_host
            || previous.4 != node.sni
            || previous.5 != node.port
            || previous.6 != node.protocol_config)
    {
        return Err(ApiError::Conflict(format!(
            "节点正在被混合链路引用（编号：{}），只能修改名称或启用状态；更换端点请创建新节点与链路",
            path_references
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("、")
        )));
    }
    sqlx::query(
        "UPDATE nodes SET name=$2,public_host=$3,sni=$4,port=$5,protocol_config=$6,enabled=$7,settings=$8 WHERE id=$1",
    )
    .bind(id)
    .bind(&node.name)
    .bind(&node.public_host)
    .bind(&node.sni)
    .bind(node.port)
    .bind(&node.protocol_config)
    .bind(node.enabled)
    .bind(&node.settings)
    .execute(&mut *transaction)
    .await
    .map_err(port_database_error)?;
    if previous
        != (
            node.enabled,
            node.settings.clone(),
            node.name.clone(),
            node.public_host.clone(),
            node.sni.clone(),
            node.port,
            node.protocol_config.clone(),
        )
    {
        business::mark_dirty(&mut transaction, &[server_id]).await?;
    }
    transaction.commit().await?;
    Ok(Json(node.view()?))
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let mut transaction = state.pool.begin().await?;
    super::entitlements::lock(&mut transaction).await?;
    let server_id: i64 =
        sqlx::query_scalar("SELECT server_id FROM nodes WHERE id=$1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(ApiError::NotFound)?;
    business::lock_server(&mut transaction, server_id).await?;
    let references:Vec<i64>=sqlx::query_scalar("SELECT DISTINCT c.id FROM singbox_live_chains c LEFT JOIN singbox_chain_hops h ON h.chain_id=c.id WHERE c.path_kind='mixed' AND (c.entry_node_id=$1 OR h.managed_node_id=$1) ORDER BY c.id").bind(id).fetch_all(&mut *transaction).await?;
    if !references.is_empty() {
        return Err(ApiError::Conflict(format!(
            "请先删除引用此节点的链路（编号：{}）；链路入口请从链路详情删除",
            references
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("、")
        )));
    }
    let result = sqlx::query("UPDATE nodes SET deleted_at=$2 WHERE id=$1 AND deleted_at IS NULL")
        .bind(id)
        .bind(now_timestamp())
        .execute(&mut *transaction)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    sqlx::query("DELETE FROM accesses WHERE node_id=$1")
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    business::mark_dirty(&mut transaction, &[server_id]).await?;
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

fn validate_port(port: i64) -> ApiResult<i32> {
    if !(1..=65535).contains(&port) {
        return Err(ApiError::BadRequest(
            "节点端口必须为 1 至 65535 的整数".into(),
        ));
    }
    if matches!(port, 18085 | 18086) {
        return Err(ApiError::BadRequest(
            "端口 18085 和 18086 已保留给本地统计与路径验证接口".into(),
        ));
    }
    Ok(port as i32)
}

async fn ensure_port_available(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    server_id: i64,
    port: i32,
    node_id: Option<i64>,
) -> ApiResult<()> {
    let occupied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM nodes WHERE server_id=$1 AND port=$2 AND deleted_at IS NULL AND ($3::bigint IS NULL OR id<>$3))")
        .bind(server_id)
        .bind(port)
        .bind(node_id)
        .fetch_one(&mut **transaction)
        .await?;
    if occupied {
        return Err(ApiError::Conflict("此服务器的端口已被其他节点使用".into()));
    }
    Ok(())
}

fn port_database_error(error: sqlx::Error) -> ApiError {
    if error.as_database_error().is_some_and(|error| {
        error.is_unique_violation() && error.constraint() == Some("nodes_active_port_idx")
    }) {
        ApiError::Conflict("此服务器的端口已被其他节点使用".into())
    } else {
        error.into()
    }
}

pub(crate) async fn validate_server_config(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeRow,
) -> ApiResult<()> {
    let query = format!(
        "SELECT {NODE_COLUMNS} FROM nodes n WHERE n.server_id=$1 AND n.deleted_at IS NULL AND n.id<>$2 ORDER BY n.id"
    );
    let rows = sqlx::query_as::<_, NodeRow>(&query)
        .bind(node.server_id)
        .bind(node.id)
        .fetch_all(&mut **transaction)
        .await?;
    let mut nodes = rows
        .iter()
        .map(|row| row.model(vec![]))
        .collect::<anyhow::Result<Vec<_>>>()?;
    nodes.push(node.model(vec![])?);
    sinan_compiler::compile_server(&nodes)
        .map_err(|error| ApiError::BadRequest(format!("服务器节点配置冲突：{error}")))?;
    Ok(())
}
