use crate::{
    auth::require_admin,
    business::{self, NodeRow, NodeView, NODE_COLUMNS},
    error::{ApiError, ApiResult},
    AppState,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::Deserialize;
use sinan_protocol::now_timestamp;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateNode {
    pub name: String,
    pub server_id: i64,
    pub public_host: String,
    pub sni: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateNode {
    pub name: Option<String>,
    pub public_host: Option<String>,
    pub sni: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<NodeView>>> {
    require_admin(&state, &headers).await?;
    let query = format!("SELECT {NODE_COLUMNS} FROM nodes n JOIN servers s ON s.id=n.server_id WHERE n.deleted_at IS NULL AND s.deleted_at IS NULL ORDER BY n.id");
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
    let query = format!("SELECT {NODE_COLUMNS} FROM nodes n JOIN servers s ON s.id=n.server_id WHERE n.id=$1 AND n.deleted_at IS NULL AND s.deleted_at IS NULL");
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
    let (private_key, public_key) = business::generate_reality_keypair();
    let mut node = NodeRow {
        id: 1,
        name: business::name(&request.name)?,
        server_id: request.server_id,
        protocol: "vless-reality".into(),
        port: 20000,
        public_host: request.public_host,
        sni: request.sni,
        private_key,
        public_key,
        short_id: business::short_id(),
    };
    business::validate_node(&node)?;
    let mut transaction = state.pool.begin().await?;
    business::lock_server(&mut transaction, request.server_id).await?;
    node.port = sqlx::query_scalar::<_, i32>("SELECT candidate.port FROM generate_series(20000,29999) AS candidate(port) WHERE NOT EXISTS(SELECT 1 FROM nodes WHERE server_id=$1 AND deleted_at IS NULL AND nodes.port=candidate.port) ORDER BY candidate.port LIMIT 1").bind(node.server_id).fetch_optional(&mut *transaction).await?.ok_or_else(|| ApiError::Conflict("服务器没有可分配端口".into()))?;
    let query = format!("INSERT INTO nodes AS n (name,server_id,protocol,port,public_host,sni,private_key,public_key,short_id) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING {NODE_COLUMNS}");
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
        .fetch_one(&mut *transaction)
        .await?;
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
    if request.name.is_none() && request.public_host.is_none() && request.sni.is_none() {
        return Err(ApiError::BadRequest("至少提供一个修改字段".into()));
    }
    let mut transaction = state.pool.begin().await?;
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
    if let Some(name) = request.name {
        node.name = business::name(&name)?;
    }
    if let Some(host) = request.public_host {
        node.public_host = host;
    }
    if let Some(sni) = request.sni {
        node.sni = sni;
    }
    business::validate_node(&node)?;
    sqlx::query("UPDATE nodes SET name=$2,public_host=$3,sni=$4 WHERE id=$1")
        .bind(id)
        .bind(&node.name)
        .bind(&node.public_host)
        .bind(&node.sni)
        .execute(&mut *transaction)
        .await?;
    business::mark_dirty(&mut transaction, &[server_id]).await?;
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
    let server_id: i64 =
        sqlx::query_scalar("SELECT server_id FROM nodes WHERE id=$1 AND deleted_at IS NULL")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(ApiError::NotFound)?;
    business::lock_server(&mut transaction, server_id).await?;
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
