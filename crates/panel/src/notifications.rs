mod evaluation;
mod events;
mod resources;
pub mod rules;
mod telegram;
mod template;
use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
    settings::Settings,
};
use axum::{Json, extract::State, http::HeaderMap};
pub use evaluation::evaluate;
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
pub use template::valid as valid_template;

#[derive(Serialize, FromRow)]
pub struct Event {
    id: i64,
    server_id: i64,
    server_name: String,
    last_seen: Option<i64>,
    category: String,
    message: String,
    details: Value,
    opened_at: i64,
    resolved_at: Option<i64>,
    resolution: Option<String>,
    deliveries: Value,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Event>>> {
    auth::require_admin(&state, &headers).await?;
    Ok(Json(sqlx::query_as("SELECT e.*, COALESCE((SELECT jsonb_agg(jsonb_build_object('kind',o.kind,'status',o.status,'attempts',o.attempts,'last_error',o.last_error) ORDER BY o.id) FROM notification_outbox o WHERE o.event_id=e.id),'[]'::jsonb) AS deliveries FROM server_alert_events e ORDER BY e.id DESC LIMIT 200").fetch_all(&state.pool).await?))
}

pub(super) async fn lock_settings(tx: &mut Transaction<'_, Postgres>) -> anyhow::Result<Settings> {
    let value: Value =
        sqlx::query_scalar("SELECT settings FROM panel_settings WHERE singleton FOR UPDATE")
            .fetch_one(&mut **tx)
            .await?;
    Ok(serde_json::from_value(value)?)
}

pub async fn test_telegram(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    auth::require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    let settings = lock_settings(&mut tx).await?;
    if settings.telegram_token.is_empty() || settings.telegram_chat_id.is_empty() {
        return Err(ApiError::BadRequest("请先保存机器人令牌与会话 ID".into()));
    }
    let now = sinan_protocol::now_timestamp();
    let previous: i64 = sqlx::query_scalar(
        "SELECT sent_at FROM notification_test_limit WHERE singleton FOR UPDATE",
    )
    .fetch_one(&mut *tx)
    .await?;
    if now.saturating_sub(previous) < 30 {
        return Err(ApiError::Busy);
    }
    sqlx::query("UPDATE notification_test_limit SET sent_at=$1 WHERE singleton")
        .bind(now)
        .execute(&mut *tx)
        .await?;
    let timestamp: String = sqlx::query_scalar(
        "SELECT to_char(to_timestamp($1) AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') || ' UTC'",
    )
    .bind(now as f64)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    let message = template::render(
        &settings.telegram_template,
        [
            "测试通知",
            "示例服务器",
            "这是一条手动测试消息，用于确认 Telegram 通知配置。",
            &timestamp,
            "测试",
        ],
    );
    telegram::send(
        &settings.telegram_token,
        &settings.telegram_chat_id,
        settings.telegram_thread_id,
        &message,
    )
    .await
    .map_err(|failure| ApiError::BadRequest(failure.message))?;
    Ok(Json(serde_json::json!({"sent":true})))
}

pub async fn dispatch(pool: &PgPool, now: i64) -> anyhow::Result<()> {
    dispatch_with(pool, now, |settings, message| async move {
        telegram::send(
            &settings.telegram_token,
            &settings.telegram_chat_id,
            settings.telegram_thread_id,
            &message,
        )
        .await
    })
    .await
}

async fn dispatch_with<F, Fut>(pool: &PgPool, now: i64, mut send: F) -> anyhow::Result<()>
where
    F: FnMut(Settings, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), telegram::Failure>>,
{
    for _ in 0..4 {
        let mut tx = pool.begin().await?;
        let value: Value =
            sqlx::query_scalar("SELECT settings FROM panel_settings WHERE singleton FOR SHARE")
                .fetch_one(&mut *tx)
                .await?;
        let settings: Settings = serde_json::from_value(value)?;
        if !settings.telegram_ready() {
            return Ok(());
        }
        let row: Option<(i64,String,i32)> = sqlx::query_as(
            "SELECT o.id,o.message,o.attempts FROM notification_outbox o
             JOIN server_alert_events e ON e.id=o.event_id JOIN servers s ON s.id=e.server_id
             WHERE o.status='pending' AND o.next_attempt_at<=$1 AND s.deleted_at IS NULL
             AND (e.category<>'offline' OR ($2 AND COALESCE(s.asset_settings->>'offline_notify','true')<>'false'))
             AND NOT EXISTS(SELECT 1 FROM notification_outbox earlier WHERE earlier.event_id=o.event_id AND earlier.id<o.id AND earlier.status='pending')
             ORDER BY o.id LIMIT 1 FOR UPDATE OF o SKIP LOCKED")
            .bind(now).bind(settings.offline_alerts).fetch_optional(&mut *tx).await?;
        let Some((id, message, attempts)) = row else {
            return Ok(());
        };
        let result = send(settings, message).await;
        let (status, error, delay) = match result {
            Ok(()) => ("sent", None, 0),
            Err(failure) => (
                if attempts >= 7 { "failed" } else { "pending" },
                Some(failure.message),
                failure
                    .retry_after
                    .unwrap_or(30 * (1_i64 << attempts.min(7))),
            ),
        };
        sqlx::query("UPDATE notification_outbox SET status=$2,last_error=$3,attempts=attempts+1,next_attempt_at=$4 WHERE id=$1")
            .bind(id).bind(status).bind(error).bind(now+delay).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    #[sqlx::test]
    async fn outbox_retries_in_order_and_concurrent_workers_do_not_duplicate(
        pool: PgPool,
    ) -> anyhow::Result<()> {
        let config = Settings {
            telegram_enabled: true,
            telegram_chat_id: "-100000".into(),
            telegram_token: "123:TEST_ONLY_SECRET_00000000000".into(),
            ..Default::default()
        };
        sqlx::query("UPDATE panel_settings SET settings=$1")
            .bind(serde_json::to_value(config)?)
            .execute(&pool)
            .await?;
        let id: i64 =
            sqlx::query_scalar("INSERT INTO servers(name) VALUES('outbox fixture') RETURNING id")
                .fetch_one(&pool)
                .await?;
        let event: i64 = sqlx::query_scalar("INSERT INTO server_alert_events(server_id,server_name,last_seen,opened_at,resolved_at,resolution) VALUES($1,'outbox fixture',0,1,2,'recovered') RETURNING id").bind(id).fetch_one(&pool).await?;
        let mut tx = pool.begin().await?;
        let settings = lock_settings(&mut tx).await?;
        events::enqueue(&mut tx, &settings, event, "offline", 100).await?;
        events::enqueue(&mut tx, &settings, event, "recovery", 100).await?;
        tx.commit().await?;
        dispatch_with(&pool, 100, |_, _| async {
            Err(telegram::Failure {
                message: "temporary failure".into(),
                retry_after: Some(123),
            })
        })
        .await?;
        let rows: Vec<(String, i32, i64)> = sqlx::query_as(
            "SELECT status,attempts,next_attempt_at FROM notification_outbox ORDER BY id",
        )
        .fetch_all(&pool)
        .await?;
        assert_eq!(
            rows,
            vec![("pending".into(), 1, 223), ("pending".into(), 0, 100)]
        );
        let count = Arc::new(AtomicUsize::new(0));
        let send = |_: Settings, _: String| {
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                Ok(())
            }
        };
        let (a, b) = tokio::join!(
            dispatch_with(&pool, 223, send),
            dispatch_with(&pool, 223, send)
        );
        a?;
        b?;
        assert_eq!(count.load(Ordering::SeqCst), 2);
        let statuses: Vec<String> =
            sqlx::query_scalar("SELECT status FROM notification_outbox ORDER BY id")
                .fetch_all(&pool)
                .await?;
        assert_eq!(statuses, vec!["sent", "sent"]);
        dispatch_with(&pool, 500, send).await?;
        assert_eq!(count.load(Ordering::SeqCst), 2);
        Ok(())
    }
}
