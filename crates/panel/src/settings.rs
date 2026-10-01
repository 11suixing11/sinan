use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub public_dashboard: bool,
    pub offline_alerts: bool,
    pub offline_minutes: u16,
    pub telegram_enabled: bool,
    pub telegram_chat_id: String,
    pub telegram_token: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            public_dashboard: false,
            offline_alerts: true,
            offline_minutes: 5,
            telegram_enabled: false,
            telegram_chat_id: String::new(),
            telegram_token: String::new(),
        }
    }
}

impl Settings {
    fn view(&self) -> Value {
        json!({"public_dashboard":self.public_dashboard,"offline_alerts":self.offline_alerts,
            "offline_minutes":self.offline_minutes,"telegram_enabled":self.telegram_enabled,
            "telegram_chat_id":self.telegram_chat_id,"telegram_token_configured":!self.telegram_token.is_empty()})
    }
    pub fn telegram_ready(&self) -> bool {
        self.offline_alerts
            && self.telegram_enabled
            && !self.telegram_token.is_empty()
            && !self.telegram_chat_id.is_empty()
    }
}

pub async fn read(pool: &PgPool) -> ApiResult<Settings> {
    let value: Value = sqlx::query_scalar("SELECT settings FROM panel_settings WHERE singleton")
        .fetch_one(pool)
        .await?;
    serde_json::from_value(value).map_err(|error| ApiError::Internal(error.into()))
}

pub async fn get(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    auth::require_admin(&state, &headers).await?;
    Ok(Json(read(&state.pool).await?.view()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Update {
    public_dashboard: bool,
    offline_alerts: bool,
    offline_minutes: u16,
    telegram_enabled: bool,
    telegram_chat_id: String,
    telegram_token: Option<String>,
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<Update>,
) -> ApiResult<Json<Value>> {
    auth::require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    let value: Value =
        sqlx::query_scalar("SELECT settings FROM panel_settings WHERE singleton FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
    let old: Settings = serde_json::from_value(value).map_err(anyhow::Error::from)?;
    let settings = Settings {
        public_dashboard: update.public_dashboard,
        offline_alerts: update.offline_alerts,
        offline_minutes: update.offline_minutes,
        telegram_enabled: update.telegram_enabled,
        telegram_chat_id: update.telegram_chat_id.trim().into(),
        telegram_token: update
            .telegram_token
            .map(|token| token.trim().into())
            .unwrap_or(old.telegram_token),
    };
    let valid_token = settings.telegram_token.is_empty()
        || settings
            .telegram_token
            .split_once(':')
            .is_some_and(|(id, secret)| {
                !id.is_empty()
                    && id.bytes().all(|b| b.is_ascii_digit())
                    && secret.len() >= 20
                    && secret
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
            });
    let chat = &settings.telegram_chat_id;
    let valid_chat = chat.is_empty()
        || chat.parse::<i64>().is_ok()
        || chat.strip_prefix('@').is_some_and(|v| {
            !v.is_empty() && v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        });
    if !(2..=1440).contains(&settings.offline_minutes)
        || settings.telegram_token.len() > 256
        || !valid_token
        || !valid_chat
        || chat.len() > 128
        || (settings.telegram_enabled && (settings.telegram_token.is_empty() || chat.is_empty()))
    {
        return Err(ApiError::BadRequest(
            "离线阈值需为 2–1440 分钟；启用 Telegram 时需填写有效机器人令牌和会话 ID".into(),
        ));
    }
    sqlx::query("UPDATE panel_settings SET settings=$1 WHERE singleton")
        .bind(json!(settings))
        .execute(&mut *tx)
        .await?;
    // Do not deliver stale notifications to a newly configured recipient.
    if !settings.telegram_ready() || old.telegram_chat_id != settings.telegram_chat_id {
        sqlx::query("UPDATE notification_outbox SET status='cancelled' WHERE status='pending'")
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(Json(settings.view()))
}
