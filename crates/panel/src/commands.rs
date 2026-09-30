use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sinan_protocol::{CommandResult, RemoteCommand, TaskAck, now_timestamp};
use sqlx::Row;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateCommand {
    pub command: String,
    pub timeout_secs: u32,
    pub ttl_secs: u32,
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server): Path<i64>,
    Json(request): Json<CreateCommand>,
) -> ApiResult<Json<RemoteCommand>> {
    auth::require_admin(&state, &headers).await?;
    let now = now_timestamp();
    let command = RemoteCommand {
        id: Uuid::new_v4(),
        command: request.command,
        timeout_secs: request.timeout_secs,
        expires_at: now + i64::from(request.ttl_secs),
    };
    if !command.valid() || !(1..=86400).contains(&request.ttl_secs) {
        return Err(ApiError::BadRequest("命令、超时或领取期限无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT id FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
        .bind(server)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::NotFound)?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM remote_commands WHERE server_id=$1 AND result IS NULL AND (spec->>'expires_at')::BIGINT>$2").bind(server).bind(now).fetch_one(&mut *tx).await?;
    if count >= 64 {
        return Err(ApiError::Conflict("待执行命令已达到 64 条上限".into()));
    }
    sqlx::query("INSERT INTO remote_commands(id,server_id,requested_at,spec) VALUES($1,$2,$3,$4)")
        .bind(command.id)
        .bind(server)
        .bind(now)
        .bind(serde_json::to_value(&command).map_err(anyhow::Error::from)?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(command))
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server): Path<i64>,
) -> ApiResult<Json<Vec<Value>>> {
    auth::require_admin(&state, &headers).await?;
    let rows = sqlx::query("SELECT spec,result,requested_at FROM remote_commands WHERE server_id=$1 ORDER BY requested_at DESC,id DESC LIMIT 100").bind(server).fetch_all(&state.pool).await?;
    Ok(Json(rows.iter().map(|row| json!({"spec":row.get::<Value,_>("spec"),"result":row.get::<Option<Value>,_>("result"),"requested_at":row.get::<i64,_>("requested_at")})).collect()))
}

pub async fn pending(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<RemoteCommand>>> {
    let server = auth::require_agent(&state, &headers).await?;
    let rows: Vec<Value> = sqlx::query_scalar("SELECT spec FROM remote_commands WHERE server_id=$1 AND result IS NULL AND (spec->>'expires_at')::BIGINT>$2 ORDER BY requested_at,id LIMIT 64").bind(server).bind(now_timestamp()).fetch_all(&state.pool).await?;
    Ok(Json(
        rows.into_iter()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()
            .map_err(anyhow::Error::from)?,
    ))
}

pub async fn complete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(result): Json<CommandResult>,
) -> ApiResult<Json<TaskAck>> {
    let server = auth::require_agent(&state, &headers).await?;
    if id != result.id
        || result.stdout.len() > 256 * 1024
        || result.stderr.len() > 256 * 1024
        || result.finished_at <= 0
        || result.finished_at > now_timestamp() + 60
    {
        return Err(ApiError::BadRequest("命令结果标识、时间或输出无效".into()));
    }
    let value = serde_json::to_value(&result).map_err(anyhow::Error::from)?;
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&result).map_err(anyhow::Error::from)?)
    );
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "SELECT result_digest FROM remote_commands WHERE id=$1 AND server_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(server)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    if let Some(previous) = row.get::<Option<String>, _>("result_digest") {
        if previous != digest {
            return Err(ApiError::Conflict("已完成命令结果不可更改".into()));
        }
    } else {
        sqlx::query("UPDATE remote_commands SET result=$2,result_digest=$3 WHERE id=$1")
            .bind(id)
            .bind(value)
            .bind(digest)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Json(TaskAck { ids: vec![id] }))
}
