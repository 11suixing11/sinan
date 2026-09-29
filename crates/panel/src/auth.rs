use crate::{
    error::{ApiError, ApiResult},
    AppState,
};
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sinan_protocol::now_timestamp;
use sqlx::PgPool;

const ADMIN_SESSION_SECONDS: i64 = 86_400;

pub fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn hash_token(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub async fn ensure_admin(pool: &PgPool, password: Option<&str>) -> anyhow::Result<()> {
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM admins WHERE id = 1)")
        .fetch_one(pool)
        .await?;
    if exists {
        return Ok(());
    }
    let password = password
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            anyhow::anyhow!("SINAN_ADMIN_PASSWORD is required for the initial administrator")
        })?
        .to_owned();
    if password.len() > 1024 {
        anyhow::bail!("initial administrator password exceeds 1024 bytes");
    }
    let hash = tokio::task::spawn_blocking(move || {
        let mut salt = [0_u8; 16];
        OsRng.fill_bytes(&mut salt);
        let salt =
            SaltString::encode_b64(&salt).map_err(|error| anyhow::anyhow!(error.to_string()))?;
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    })
    .await??;
    sqlx::query(
        "INSERT INTO admins (id, password_hash) VALUES (1, $1) ON CONFLICT (id) DO NOTHING",
    )
    .bind(hash)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn require_admin(state: &AppState, headers: &HeaderMap) -> ApiResult<i64> {
    let token = cookie_token(headers).ok_or(ApiError::Unauthorized)?;
    sqlx::query_scalar::<_, i64>("SELECT admin_id FROM sessions WHERE token_hash = $1 AND admin_id IS NOT NULL AND expires_at > $2")
        .bind(hash_token(token)).bind(now_timestamp()).fetch_optional(&state.pool).await?.ok_or(ApiError::Unauthorized)
}

pub async fn require_agent(state: &AppState, headers: &HeaderMap) -> ApiResult<i64> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .filter(|value| !value.is_empty() && value.len() <= 512)
        .ok_or(ApiError::Unauthorized)?;
    sqlx::query_scalar::<_, i64>("SELECT sessions.server_id FROM sessions JOIN servers ON servers.id = sessions.server_id WHERE sessions.token_hash = $1 AND sessions.server_id IS NOT NULL AND sessions.expires_at > $2 AND servers.deleted_at IS NULL")
        .bind(hash_token(token)).bind(now_timestamp()).fetch_optional(&state.pool).await?.ok_or(ApiError::Unauthorized)
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Response> {
    let _permit = state
        .login_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Busy)?;
    if request.password.len() > 1024 || request.password.is_empty() {
        return Err(ApiError::Unauthorized);
    }
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM admins WHERE id = 1")
        .fetch_one(&state.pool)
        .await?;
    let verified = tokio::task::spawn_blocking(move || {
        PasswordHash::new(&hash).is_ok_and(|hash| {
            Argon2::default()
                .verify_password(request.password.as_bytes(), &hash)
                .is_ok()
        })
    })
    .await
    .map_err(anyhow::Error::from)?;
    if !verified {
        return Err(ApiError::Unauthorized);
    }
    let token = random_token();
    let now = now_timestamp();
    let mut transaction = state.pool.begin().await?;
    sqlx::query("DELETE FROM sessions WHERE expires_at <= $1")
        .bind(now)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("INSERT INTO sessions (token_hash, admin_id, expires_at) VALUES ($1, 1, $2)")
        .bind(hash_token(&token))
        .bind(now + ADMIN_SESSION_SECONDS)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    let mut response = Json(json!({"id": 1})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        session_cookie(&state, &token, ADMIN_SESSION_SECONDS)?,
    );
    Ok(response)
}

pub async fn logout(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Response> {
    if let Some(token) = cookie_token(&headers) {
        sqlx::query("DELETE FROM sessions WHERE token_hash = $1 AND admin_id IS NOT NULL")
            .bind(hash_token(token))
            .execute(&state.pool)
            .await?;
    }
    let mut response = Json(json!({"ok": true})).into_response();
    response
        .headers_mut()
        .insert(header::SET_COOKIE, session_cookie(&state, "", 0)?);
    Ok(response)
}

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let id = require_admin(&state, &headers).await?;
    Ok(Json(json!({"id": id})))
}

fn session_cookie(state: &AppState, token: &str, lifetime: i64) -> ApiResult<HeaderValue> {
    let secure = if state.config.public_url.starts_with("https://") {
        "; Secure"
    } else {
        ""
    };
    HeaderValue::from_str(&format!(
        "sinan_session={token}; HttpOnly; SameSite=Strict; Path=/; Max-Age={lifetime}{secure}"
    ))
    .map_err(|error| ApiError::Internal(anyhow::Error::from(error)))
}

fn cookie_token(headers: &HeaderMap) -> Option<&str> {
    let mut token = None;
    for header in headers.get_all(header::COOKIE) {
        for cookie in header.to_str().ok()?.split(';') {
            let Some((name, value)) = cookie.trim().split_once('=') else {
                continue;
            };
            if name == "sinan_session" {
                if token.is_some() || value.is_empty() || value.len() > 512 {
                    return None;
                }
                token = Some(value);
            }
        }
    }
    token
}
