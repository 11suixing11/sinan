use super::{
    MAX_RULES, editable, load,
    model::{self, Config, Rule},
    worker,
};
use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/plugins/ddns/rules", get(list).post(create))
        .route("/api/plugins/ddns/rules/{id}", patch(update).delete(remove))
        .route("/api/plugins/ddns/rules/{id}/sync", post(sync))
        .merge(super::settings::routes())
}

async fn list(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Vec<Value>>> {
    auth::require_admin(&state, &headers).await?;
    let rules = sqlx::query_as::<_, Rule>("SELECT * FROM ddns_rules ORDER BY id")
        .fetch_all(&state.pool)
        .await?;
    let mut result = Vec::new();
    for rule in rules {
        result.push(model::view(&state.pool, rule).await?);
    }
    Ok(Json(result))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Write {
    config: Config,
    api_token: Option<String>,
    revision: Option<i64>,
}

async fn valid_server(state: &AppState, config: &Config) -> ApiResult<()> {
    let server = model::observation(&state.pool, config.server_id).await?;
    if !server.plugin_enabled {
        return Err(ApiError::Conflict("请先为该服务器启用 DDNS 插件".into()));
    }
    if server.deleted_at.is_some() || server.retiring {
        return Err(ApiError::Conflict(
            "无法绑定已删除或正在退役的服务器".into(),
        ));
    }
    Ok(())
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut input): Json<Write>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    auth::require_admin(&state, &headers).await?;
    input.config.normalize()?;
    valid_server(&state, &input.config).await?;
    let token = model::token(input.api_token.as_deref().unwrap_or_default())?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(739104823)")
        .execute(&mut *tx)
        .await?;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM ddns_rules")
        .fetch_one(&mut *tx)
        .await?;
    if count >= MAX_RULES {
        return Err(ApiError::Conflict("最多配置 32 条 DDNS 规则".into()));
    }
    let id = Uuid::new_v4();
    let result =
        sqlx::query("INSERT INTO ddns_rules(id,server_id,config,api_token) VALUES($1,$2,$3,$4)")
            .bind(id)
            .bind(input.config.server_id)
            .bind(json!(input.config))
            .bind(token)
            .execute(&mut *tx)
            .await;
    if result.as_ref().is_err_and(|error| {
        error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
    }) {
        return Err(ApiError::Conflict(
            "此 Zone、域名和记录类型已存在规则".into(),
        ));
    }
    result?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(model::view(&state.pool, load(&state.pool, id).await?).await?),
    ))
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(mut input): Json<Write>,
) -> ApiResult<Json<Value>> {
    auth::require_admin(&state, &headers).await?;
    input.config.normalize()?;
    let mut tx = state.pool.begin().await?;
    let previous = editable(&mut tx, id).await?;
    if input.revision != Some(previous.revision) {
        return Err(ApiError::Conflict("规则已被修改，请刷新后重试".into()));
    }
    if input.config.zone_id != previous.config.zone_id
        || input.config.record_name != previous.config.record_name
        || input.config.record_type != previous.config.record_type
    {
        return Err(ApiError::BadRequest(
            "Zone、域名和类型创建后固定，请另建规则".into(),
        ));
    }
    // A retired binding may still be paused or have its secret replaced.
    if input.config.enabled || input.config.server_id != previous.config.server_id {
        valid_server(&state, &input.config).await?;
    }
    let replacement = input
        .api_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(model::token)
        .transpose()?;
    sqlx::query("UPDATE ddns_rules SET server_id=$2,config=$3,api_token=COALESCE($4,api_token),revision=revision+1,next_run_at=GREATEST(0,COALESCE(attempted_at,0)+60),failures=0,status='pending',error_code=NULL,lease_id=NULL,lease_until=0 WHERE id=$1")
        .bind(id).bind(input.config.server_id).bind(json!(input.config)).bind(replacement).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(
        model::view(&state.pool, load(&state.pool, id).await?).await?,
    ))
}

async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<StatusCode> {
    auth::require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    editable(&mut tx, id).await?;
    sqlx::query("DELETE FROM ddns_rules WHERE id=$1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn sync(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<Value>> {
    auth::require_admin(&state, &headers).await?;
    worker::sync(&state.pool, id, true).await?;
    Ok(Json(
        model::view(&state.pool, load(&state.pool, id).await?).await?,
    ))
}
