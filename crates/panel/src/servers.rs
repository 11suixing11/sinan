use crate::{
    AppState,
    auth::{hash_token, random_token, require_admin},
    error::{ApiError, ApiResult},
    server_assets::AssetSettings,
    server_traffic::{self, TrafficSummary},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::VerifyingKey;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sinan_protocol::{AgentSettings, EnrollRequest, EnrollResponse, ProbeSpec, now_timestamp};
use sqlx::{FromRow, PgPool, Row};
use uuid::Uuid;

const SERVER_COLUMNS: &str = "id, name, device_public_key, static_info, last_seen, last_heartbeat_at, NULLIF(metrics_sampled_at,0) AS metrics_sampled_at, latest_metrics, agent_settings, asset_settings, manifest_rev, capabilities";

#[derive(Serialize, FromRow)]
pub struct Server {
    pub id: i64,
    pub name: String,
    pub device_public_key: Option<String>,
    pub static_info: Value,
    pub last_seen: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
    pub metrics_sampled_at: Option<i64>,
    #[serde(skip)]
    agent_settings: Value,
    #[sqlx(json)]
    pub asset_settings: AssetSettings,
    #[sqlx(skip)]
    pub traffic: Option<TrafficSummary>,
    pub latest_metrics: Value,
    pub manifest_rev: i64,
    pub capabilities: Value,
    #[sqlx(default)]
    pub online: bool,
    #[sqlx(default)]
    pub metrics_stale: bool,
}

impl Server {
    fn with_online(mut self) -> Self {
        let now = now_timestamp();
        self.asset_settings.renew(now);
        self.online = self
            .last_seen
            .is_some_and(|seen| now.saturating_sub(seen) <= 60);
        let settings =
            serde_json::from_value::<sinan_protocol::AgentSettings>(self.agent_settings.clone())
                .unwrap_or_default();
        let allowance = settings
            .sample_interval_secs
            .saturating_mul(3)
            .saturating_add(settings.upload_interval_secs.saturating_mul(2))
            .max(15);
        self.metrics_stale = self.metrics_sampled_at.is_some_and(|sampled| {
            sinan_protocol::telemetry::now_millis().saturating_sub(sampled)
                > (allowance as i64).saturating_mul(1000)
        });
        self
    }
}

#[derive(Deserialize)]
pub struct ServerRequest {
    pub name: String,
    pub asset_settings: Option<AssetSettings>,
}

#[derive(Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    #[serde(default)]
    pub agent_settings: AgentSettings,
    #[serde(default)]
    pub probes: Vec<ProbeSpec>,
    #[serde(default)]
    pub asset_settings: AssetSettings,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Server>>> {
    require_admin(&state, &headers).await?;
    let query =
        format!("SELECT {SERVER_COLUMNS} FROM servers WHERE deleted_at IS NULL ORDER BY id");
    let servers = sqlx::query_as::<_, Server>(&query)
        .fetch_all(&state.pool)
        .await?;
    let mut servers: Vec<_> = servers.into_iter().map(Server::with_online).collect();
    server_traffic::attach(&state.pool, &mut servers, now_timestamp()).await?;
    Ok(Json(servers))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut request): Json<CreateServerRequest>,
) -> ApiResult<(StatusCode, Json<Server>)> {
    require_admin(&state, &headers).await?;
    let name = valid_name(&request.name)?;
    let mut asset = request.asset_settings.normalized()?;
    asset.renew(now_timestamp());
    if !request.agent_settings.valid() {
        return Err(ApiError::BadRequest(
            "采样与上传间隔必须在 1–60 秒内，上传间隔不能小于采样间隔".into(),
        ));
    }
    if request.probes.len() > 32 {
        return Err(ApiError::BadRequest("每台服务器最多配置 32 个拨测".into()));
    }
    for spec in &mut request.probes {
        spec.id = Uuid::new_v4();
        if !spec.valid() {
            return Err(ApiError::BadRequest("拨测配置无效".into()));
        }
    }
    let mut transaction = state.pool.begin().await?;
    let query = format!(
        "INSERT INTO servers (name, agent_settings, asset_settings) VALUES ($1, $2, $3) RETURNING {SERVER_COLUMNS}"
    );
    let server = sqlx::query_as::<_, Server>(&query)
        .bind(name)
        .bind(json!(request.agent_settings))
        .bind(json!(asset))
        .fetch_one(&mut *transaction)
        .await?;
    for spec in request.probes {
        sqlx::query("INSERT INTO network_probes (id, server_id, spec) VALUES ($1, $2, $3)")
            .bind(spec.id)
            .bind(server.id)
            .bind(json!(spec))
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(server.with_online())))
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<Server>> {
    require_admin(&state, &headers).await?;
    let query =
        format!("SELECT {SERVER_COLUMNS} FROM servers WHERE id = $1 AND deleted_at IS NULL");
    let server = sqlx::query_as::<_, Server>(&query)
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    let mut server = server.with_online();
    server_traffic::attach(
        &state.pool,
        std::slice::from_mut(&mut server),
        now_timestamp(),
    )
    .await?;
    Ok(Json(server))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<ServerRequest>,
) -> ApiResult<Json<Server>> {
    require_admin(&state, &headers).await?;
    let name = valid_name(&request.name)?;
    let asset = request
        .asset_settings
        .map(|asset| {
            let mut asset = asset.normalized()?;
            asset.renew(now_timestamp());
            Ok::<_, ApiError>(json!(asset))
        })
        .transpose()?;
    let query = format!(
        "UPDATE servers SET name = $2, asset_settings=COALESCE($3,asset_settings) WHERE id = $1 AND deleted_at IS NULL RETURNING {SERVER_COLUMNS}"
    );
    let server = sqlx::query_as::<_, Server>(&query)
        .bind(id)
        .bind(name)
        .bind(asset)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    let mut server = server.with_online();
    server_traffic::attach(
        &state.pool,
        std::slice::from_mut(&mut server),
        now_timestamp(),
    )
    .await?;
    Ok(Json(server))
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<StatusCode> {
    require_admin(&state, &headers).await?;
    crate::retirement::remove(&state, id).await
}

#[derive(Deserialize, Default)]
pub struct EnrollmentQuery {
    pub agent_version: Option<String>,
}

pub async fn issue_enrollment(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<EnrollmentQuery>,
) -> ApiResult<Json<Value>> {
    require_admin(&state, &headers).await?;
    let token = random_token();
    let expires_at = now_timestamp() + 86_400;
    let mut transaction = state.pool.begin().await?;
    let exists =
        sqlx::query("SELECT id FROM servers WHERE id = $1 AND deleted_at IS NULL FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await?;
    if exists.is_none() {
        return Err(ApiError::NotFound);
    }
    sqlx::query(
        "INSERT INTO enrollment_tokens (token_hash, server_id, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(hash_token(&token))
    .bind(id)
    .bind(expires_at)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    let selection =
        crate::installation::select(&state, query.agent_version.as_deref(), &token).await;
    let (install_command, installation, warning) = match selection {
        Ok(installation) => (
            Some(installation.install_command.clone()),
            Some(json!(installation)),
            None,
        ),
        Err(_) => (
            None,
            None,
            Some("请先导入协议兼容且已签名的 Agent Release，再获取安装命令"),
        ),
    };
    Ok(Json(
        json!({"token": token, "expires_at": expires_at, "install_command": install_command,
        "installation": installation, "warning": warning}),
    ))
}

pub async fn validate_enrollment(pool: &PgPool, token: &str) -> ApiResult<i64> {
    if token.is_empty() || token.len() > 512 {
        return Err(ApiError::Unauthorized);
    }
    sqlx::query_scalar::<_, i64>("SELECT enrollment_tokens.server_id FROM enrollment_tokens JOIN servers ON servers.id = enrollment_tokens.server_id WHERE enrollment_tokens.token_hash = $1 AND enrollment_tokens.expires_at > $2 AND enrollment_tokens.consumed_at IS NULL AND servers.deleted_at IS NULL")
        .bind(hash_token(token)).bind(now_timestamp()).fetch_optional(pool).await?.ok_or(ApiError::Unauthorized)
}

pub async fn enroll(
    State(state): State<AppState>,
    Json(request): Json<EnrollRequest>,
) -> ApiResult<Json<EnrollResponse>> {
    validate_public_key(&request.device_public_key)?;
    if request.token.is_empty() || request.token.len() > 512 {
        return Err(ApiError::Unauthorized);
    }
    let now = now_timestamp();
    let token_hash = hash_token(&request.token);
    let mut transaction = state.pool.begin().await?;
    let row = sqlx::query("SELECT e.server_id, s.device_public_key FROM enrollment_tokens e JOIN servers s ON s.id = e.server_id WHERE e.token_hash = $1 AND e.expires_at > $2 AND e.consumed_at IS NULL AND s.deleted_at IS NULL FOR UPDATE OF e, s")
        .bind(&token_hash).bind(now).fetch_optional(&mut *transaction).await?.ok_or(ApiError::Unauthorized)?;
    let id: i64 = row.try_get("server_id")?;
    let previous_key: Option<String> = row.try_get("device_public_key")?;
    if previous_key
        .as_ref()
        .is_some_and(|key| key != &request.device_public_key)
    {
        return Err(ApiError::Conflict(
            "服务器已经注册，设备公钥不能更换".into(),
        ));
    }
    sqlx::query("UPDATE servers SET device_public_key = $2, static_info = $3 WHERE id = $1")
        .bind(id)
        .bind(&request.device_public_key)
        .bind(json!(request.static_info))
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE enrollment_tokens SET consumed_at = $2 WHERE token_hash = $1")
        .bind(token_hash)
        .bind(now)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(Json(EnrollResponse { server_id: id }))
}

fn valid_name(value: &str) -> ApiResult<&str> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 128 || value.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(
            "服务器名称需为 1 至 128 个字符，且不能包含控制字符".into(),
        ));
    }
    Ok(value)
}

fn validate_public_key(value: &str) -> ApiResult<()> {
    let error =
        || ApiError::BadRequest("设备公钥必须是 URL-safe 无填充 base64 编码的 ed25519 公钥".into());
    let bytes = URL_SAFE_NO_PAD.decode(value).map_err(|_| error())?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| error())?;
    let key = VerifyingKey::from_bytes(&bytes).map_err(|_| error())?;
    if key.is_weak() {
        return Err(error());
    }
    Ok(())
}
