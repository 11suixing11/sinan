use super::{telegram, webhook};
use crate::settings::Settings;
use serde_json::Value;
use sqlx::PgPool;

pub async fn dispatch(pool: &PgPool, now: i64) -> anyhow::Result<()> {
    dispatch_with(pool, now, |settings, channel, message| async move {
        if channel == "webhook" {
            let config = settings.webhook.expect("ready channel has settings");
            webhook::send(&config, &message).await
        } else {
            telegram::send(
                &settings.telegram_token,
                &settings.telegram_chat_id,
                settings.telegram_thread_id,
                &message,
            )
            .await
        }
    })
    .await
}

async fn dispatch_with<F, Fut>(pool: &PgPool, now: i64, send: F) -> anyhow::Result<()>
where
    F: FnMut(Settings, String, String) -> Fut + Clone,
    Fut: std::future::Future<Output = Result<(), telegram::Failure>>,
{
    tokio::try_join!(
        dispatch_channel(pool, now, "telegram", send.clone()),
        dispatch_channel(pool, now, "webhook", send)
    )?;
    Ok(())
}

async fn dispatch_channel<F, Fut>(
    pool: &PgPool,
    now: i64,
    channel: &str,
    mut send: F,
) -> anyhow::Result<()>
where
    F: FnMut(Settings, String, String) -> Fut,
    Fut: std::future::Future<Output = Result<(), telegram::Failure>>,
{
    for _ in 0..4 {
        let mut tx = pool.begin().await?;
        // Hold the shared settings lock through the bounded request so credentials cannot
        // change between checking the destination and attempting delivery.
        let value: Value =
            sqlx::query_scalar("SELECT settings FROM panel_settings WHERE singleton FOR SHARE")
                .fetch_one(&mut *tx)
                .await?;
        let settings: Settings = serde_json::from_value(value)?;
        if !settings.telegram_ready() && !settings.webhook_ready() {
            return Ok(());
        }
        let row: Option<(i64,String,String,i32)> = sqlx::query_as(
            "SELECT o.id,o.channel,o.message,o.attempts FROM notification_outbox o
             JOIN server_alert_events e ON e.id=o.event_id JOIN servers s ON s.id=e.server_id
             WHERE o.status='pending' AND o.next_attempt_at<=$1 AND s.deleted_at IS NULL AND o.channel=$5
             AND (e.category<>'offline' OR ($2 AND COALESCE(s.asset_settings->>'offline_notify','true')<>'false'))
             AND ((o.channel='telegram' AND $3) OR (o.channel='webhook' AND $4))
             AND NOT EXISTS(SELECT 1 FROM notification_outbox earlier WHERE earlier.event_id=o.event_id AND earlier.channel=o.channel AND earlier.id<o.id AND earlier.status='pending')
             ORDER BY o.next_attempt_at,o.id LIMIT 1 FOR UPDATE OF o SKIP LOCKED")
            .bind(now).bind(settings.offline_alerts).bind(settings.telegram_ready()).bind(settings.webhook_ready()).bind(channel).fetch_optional(&mut *tx).await?;
        let Some((id, channel, message, attempts)) = row else {
            return Ok(());
        };
        let (status, error, delay) = match send(settings, channel, message).await {
            Ok(()) => ("sent", None, 0),
            Err(failure) => (
                if attempts >= 7 { "failed" } else { "pending" },
                Some(failure.message),
                failure
                    .retry_after
                    .unwrap_or(30 * (1_i64 << attempts.clamp(0, 7)))
                    .clamp(1, 86400),
            ),
        };
        sqlx::query("UPDATE notification_outbox SET status=$2,last_error=$3,attempts=attempts+1,next_attempt_at=$4,last_attempt_at=$5,delivered_at=CASE WHEN $2='sent' THEN $5 ELSE NULL END WHERE id=$1")
            .bind(id).bind(status).bind(error).bind(now.saturating_add(delay)).bind(now).execute(&mut *tx).await?;
        tx.commit().await?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "outbox_tests.rs"]
mod tests;
