use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Serialize;
use sqlx::FromRow;

#[derive(Serialize, FromRow)]
pub struct DeploymentStatus {
    pub module: String,
    pub target_rev: i64,
    pub applied_rev: i64,
    pub last_result_rev: i64,
    pub healthy: bool,
    pub last_error: Option<String>,
    pub updated_at: i64,
}

#[derive(Serialize, FromRow)]
pub struct DeploymentHistory {
    pub module: String,
    pub rev: i64,
    pub bundle_sha256: String,
    pub created_at: i64,
}

#[derive(Serialize)]
pub struct DeploymentView {
    pub status: Option<DeploymentStatus>,
    pub history: Vec<DeploymentHistory>,
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<DeploymentView>> {
    auth::require_admin(&state, &headers).await?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM servers WHERE id=$1 AND deleted_at IS NULL)",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;
    if !exists {
        return Err(ApiError::NotFound);
    }
    let status = sqlx::query_as("SELECT module,target_rev,applied_rev,last_result_rev,healthy,last_error,updated_at FROM server_module_status WHERE server_id=$1 AND module='singbox'").bind(id).fetch_optional(&state.pool).await?;
    let history = sqlx::query_as("SELECT module,rev,bundle_sha256,created_at FROM deployments WHERE server_id=$1 ORDER BY rev DESC LIMIT 100").bind(id).fetch_all(&state.pool).await?;
    Ok(Json(DeploymentView { status, history }))
}
