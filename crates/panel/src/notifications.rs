mod telegram;
use crate::{AppState, auth, error::ApiResult, settings::Settings};
use axum::{Json, extract::State, http::HeaderMap};
use serde::Serialize;
use serde_json::Value;
use sqlx::{FromRow, PgPool, Postgres, Transaction};

#[derive(Serialize, FromRow)]
pub struct Event {
    id: i64,
    server_id: i64,
    server_name: String,
    last_seen: i64,
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
    Ok(Json(sqlx::query_as("SELECT e.*, COALESCE((SELECT jsonb_agg(jsonb_build_object('kind',o.kind,'status',o.status,'attempts',o.attempts,'last_error',o.last_error) ORDER BY o.id) FROM notification_outbox o WHERE o.event_id=e.id),'[]'::jsonb) AS deliveries FROM server_offline_events e ORDER BY e.id DESC LIMIT 200").fetch_all(&state.pool).await?))
}

async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    event: i64,
    kind: &str,
    name: &str,
    now: i64,
) -> anyhow::Result<()> {
    let title = if kind == "offline" {
        "服务器离线"
    } else {
        "服务器恢复在线"
    };
    sqlx::query("INSERT INTO notification_outbox(event_id,kind,message,next_attempt_at) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(event).bind(kind).bind(format!("司南 · {title}\n{name}\n事件 #{event}"))
        .bind(now).execute(&mut **tx).await?;
    Ok(())
}

#[derive(FromRow)]
struct Candidate {
    id: i64,
    name: String,
    last_seen: Option<i64>,
    enabled: bool,
    event_id: Option<i64>,
}

pub async fn evaluate(pool: &PgPool, started_at: i64, now: i64) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    // Serialize alert transitions and configuration changes across workers.
    let value: Value =
        sqlx::query_scalar("SELECT settings FROM panel_settings WHERE singleton FOR UPDATE")
            .fetch_one(&mut *tx)
            .await?;
    let settings: Settings = serde_json::from_value(value)?;
    let threshold = i64::from(settings.offline_minutes) * 60;
    if now.saturating_sub(started_at) < threshold {
        return Ok(());
    }
    let candidates: Vec<Candidate> = sqlx::query_as(
        "SELECT s.id,s.name,s.last_seen,(s.deleted_at IS NULL AND COALESCE(s.asset_settings->>'offline_notify','true')<>'false') AS enabled,e.id AS event_id
         FROM servers s LEFT JOIN server_offline_events e ON e.server_id=s.id AND e.resolved_at IS NULL
         WHERE e.id IS NOT NULL OR ($2 AND s.deleted_at IS NULL AND s.last_seen IS NOT NULL AND s.last_seen<=$1
            AND COALESCE(s.asset_settings->>'offline_notify','true')<>'false') ORDER BY s.id FOR UPDATE OF s")
        .bind(now-threshold).bind(settings.offline_alerts).fetch_all(&mut *tx).await?;
    for server in candidates {
        let enabled = settings.offline_alerts && server.enabled;
        let online = server
            .last_seen
            .is_some_and(|seen| now.saturating_sub(seen) <= 60);
        if let Some(event) = server.event_id {
            if !enabled || online {
                let resolution = if online { "recovered" } else { "disabled" };
                sqlx::query(
                    "UPDATE server_offline_events SET resolved_at=$2,resolution=$3 WHERE id=$1",
                )
                .bind(event)
                .bind(now)
                .bind(resolution)
                .execute(&mut *tx)
                .await?;
                if !enabled {
                    sqlx::query("UPDATE notification_outbox SET status='cancelled' WHERE event_id=$1 AND status='pending'").bind(event).execute(&mut *tx).await?;
                } else if settings.telegram_ready() {
                    enqueue(&mut tx, event, "recovery", &server.name, now).await?;
                }
            }
        } else if enabled && !online {
            let event: i64 = sqlx::query_scalar("INSERT INTO server_offline_events(server_id,server_name,last_seen,opened_at) VALUES($1,$2,$3,$4) RETURNING id")
                .bind(server.id).bind(&server.name).bind(server.last_seen).bind(now).fetch_one(&mut *tx).await?;
            if settings.telegram_ready() {
                enqueue(&mut tx, event, "offline", &server.name, now).await?;
            }
        }
    }
    sqlx::query("DELETE FROM server_offline_events WHERE id IN (SELECT e.id FROM server_offline_events e WHERE e.resolved_at<$1 AND NOT EXISTS(SELECT 1 FROM notification_outbox o WHERE o.event_id=e.id AND o.status='pending') ORDER BY e.id LIMIT 500)")
        .bind(now-90*86400).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn dispatch(pool: &PgPool, now: i64) -> anyhow::Result<()> {
    dispatch_with(pool, now, |settings, message| async move {
        telegram::send(
            &settings.telegram_token,
            &settings.telegram_chat_id,
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
             JOIN server_offline_events e ON e.id=o.event_id JOIN servers s ON s.id=e.server_id
             WHERE o.status='pending' AND o.next_attempt_at<=$1 AND s.deleted_at IS NULL
             AND COALESCE(s.asset_settings->>'offline_notify','true')<>'false'
             AND NOT EXISTS(SELECT 1 FROM notification_outbox earlier WHERE earlier.event_id=o.event_id AND earlier.id<o.id AND earlier.status='pending')
             ORDER BY o.id LIMIT 1 FOR UPDATE OF o SKIP LOCKED")
            .bind(now).fetch_optional(&mut *tx).await?;
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
        let event: i64 = sqlx::query_scalar("INSERT INTO server_offline_events(server_id,server_name,last_seen,opened_at,resolved_at,resolution) VALUES($1,'outbox fixture',0,1,2,'recovered') RETURNING id").bind(id).fetch_one(&pool).await?;
        let mut tx = pool.begin().await?;
        enqueue(&mut tx, event, "offline", "fixture", 100).await?;
        enqueue(&mut tx, event, "recovery", "fixture", 100).await?;
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
