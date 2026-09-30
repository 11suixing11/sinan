use crate::{
    AppState,
    auth::require_admin,
    business,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Serialize, FromRow)]
pub struct AccessView {
    pub user_id: i64,
    pub node_id: i64,
    pub uuid: Uuid,
    pub stat_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantRequest {
    pub node_id: i64,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<Vec<AccessView>>> {
    require_admin(&state, &headers).await?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id=$1 AND deleted_at IS NULL)")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;
    if !exists {
        return Err(ApiError::NotFound);
    }
    let accesses = sqlx::query_as::<_, AccessView>("SELECT a.user_id,a.node_id,a.uuid,a.stat_name FROM accesses a JOIN nodes n ON n.id=a.node_id JOIN servers s ON s.id=n.server_id WHERE a.user_id=$1 AND n.deleted_at IS NULL AND s.deleted_at IS NULL ORDER BY a.node_id").bind(id).fetch_all(&state.pool).await?;
    Ok(Json(accesses))
}

pub async fn grant(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<GrantRequest>,
) -> ApiResult<Json<AccessView>> {
    require_admin(&state, &headers).await?;
    let mut transaction = state.pool.begin().await?;
    business::lock_user(&mut transaction, id).await?;
    let server_id: i64 =
        sqlx::query_scalar("SELECT server_id FROM nodes WHERE id=$1 AND deleted_at IS NULL")
            .bind(request.node_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(ApiError::NotFound)?;
    business::lock_server(&mut transaction, server_id).await?;
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM nodes WHERE id=$1 AND deleted_at IS NULL)")
            .bind(request.node_id)
            .fetch_one(&mut *transaction)
            .await?;
    if !exists {
        return Err(ApiError::NotFound);
    }
    if let Some(access) = sqlx::query_as::<_, AccessView>(
        "SELECT user_id,node_id,uuid,stat_name FROM accesses WHERE user_id=$1 AND node_id=$2",
    )
    .bind(id)
    .bind(request.node_id)
    .fetch_optional(&mut *transaction)
    .await?
    {
        transaction.commit().await?;
        return Ok(Json(access));
    }
    let access = sqlx::query_as::<_, AccessView>("INSERT INTO accesses(user_id,node_id,uuid,stat_name) VALUES($1,$2,$3,$4) RETURNING user_id,node_id,uuid,stat_name").bind(id).bind(request.node_id).bind(Uuid::new_v4()).bind(sinan_compiler::stat_name(id, request.node_id)).fetch_one(&mut *transaction).await?;
    business::mark_dirty(&mut transaction, &[server_id]).await?;
    transaction.commit().await?;
    Ok(Json(access))
}

pub async fn revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((user_id, node_id)): Path<(i64, i64)>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    let mut transaction = state.pool.begin().await?;
    business::lock_user(&mut transaction, user_id).await?;
    let server_id: i64 =
        sqlx::query_scalar("SELECT server_id FROM nodes WHERE id=$1 AND deleted_at IS NULL")
            .bind(node_id)
            .fetch_optional(&mut *transaction)
            .await?
            .ok_or(ApiError::NotFound)?;
    business::lock_server(&mut transaction, server_id).await?;
    let removed = sqlx::query("DELETE FROM accesses WHERE user_id=$1 AND node_id=$2")
        .bind(user_id)
        .bind(node_id)
        .execute(&mut *transaction)
        .await?;
    if removed.rows_affected() > 0 {
        business::mark_dirty(&mut transaction, &[server_id]).await?;
    }
    transaction.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
