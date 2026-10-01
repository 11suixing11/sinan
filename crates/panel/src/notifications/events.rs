use crate::{servers::Server, settings::Settings};
use serde_json::Value;
use sqlx::{Postgres, Transaction};

pub(super) struct Observation {
    pub key: String,
    pub category: &'static str,
    pub active: bool,
    pub message: String,
    pub details: Value,
    pub reminder_until: Option<i64>,
}

pub(super) fn title(category: &str, recovery: bool) -> &'static str {
    match (category, recovery) {
        ("offline", false) => "服务器离线",
        ("offline", true) => "服务器恢复在线",
        ("resource", false) => "资源超限告警",
        ("resource", true) => "资源使用恢复",
        ("expiry", _) => "服务器到期提醒",
        ("traffic", _) => "服务器流量提醒",
        _ => "服务器通知",
    }
}

pub(super) async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    settings: &Settings,
    event: i64,
    kind: &str,
    now: i64,
) -> anyhow::Result<()> {
    if !settings.telegram_ready() {
        return Ok(());
    }
    let (category, name, message): (String, String, String) =
        sqlx::query_as("SELECT category,server_name,message FROM server_alert_events WHERE id=$1")
            .bind(event)
            .fetch_one(&mut **tx)
            .await?;
    let timestamp: String = sqlx::query_scalar(
        "SELECT to_char(to_timestamp($1) AT TIME ZONE 'UTC','YYYY-MM-DD HH24:MI:SS') || ' UTC'",
    )
    .bind(now as f64)
    .fetch_one(&mut **tx)
    .await?;
    let message = if kind == "recovery" {
        "检测已恢复正常。"
    } else {
        &message
    };
    let text = super::template::render(
        &settings.telegram_template,
        [
            title(&category, kind == "recovery"),
            &name,
            message,
            &timestamp,
            &event.to_string(),
        ],
    );
    sqlx::query("INSERT INTO notification_outbox(event_id,kind,message,next_attempt_at) VALUES($1,$2,$3,$4) ON CONFLICT DO NOTHING")
        .bind(event).bind(kind).bind(text).bind(now).execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn observe(
    tx: &mut Transaction<'_, Postgres>,
    settings: &Settings,
    server: &Server,
    observation: Observation,
    now: i64,
) -> anyhow::Result<()> {
    let previous: Option<i64> = sqlx::query_scalar("SELECT id FROM server_alert_events WHERE server_id=$1 AND source_key=$2 AND resolved_at IS NULL")
        .bind(server.id).bind(&observation.key).fetch_optional(&mut **tx).await?;
    if let Some(event) = previous {
        if !observation.active {
            sqlx::query(
                "UPDATE server_alert_events SET resolved_at=$2,resolution='recovered' WHERE id=$1",
            )
            .bind(event)
            .bind(now)
            .execute(&mut **tx)
            .await?;
            if matches!(observation.category, "offline" | "resource") {
                enqueue(tx, settings, event, "recovery", now).await?;
            }
        }
        return Ok(());
    }
    if !observation.active {
        return Ok(());
    }
    if let Some(until) = observation.reminder_until {
        let inserted = sqlx::query("INSERT INTO alert_reminder_receipts(server_id,source_key,retain_until) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(server.id).bind(&observation.key).bind(until).execute(&mut **tx).await?.rows_affected();
        if inserted == 0 {
            return Ok(());
        }
    }
    let event: i64 = sqlx::query_scalar("INSERT INTO server_alert_events(server_id,server_name,last_seen,opened_at,category,source_key,details,message) VALUES($1,$2,$3,$4,$5,$6,$7,$8) RETURNING id")
        .bind(server.id).bind(&server.name).bind(server.last_seen).bind(now).bind(observation.category)
        .bind(observation.key).bind(observation.details).bind(observation.message).fetch_one(&mut **tx).await?;
    enqueue(
        tx,
        settings,
        event,
        if observation.category == "offline" {
            "offline"
        } else {
            "alert"
        },
        now,
    )
    .await
}

pub(super) async fn close_obsolete(
    tx: &mut Transaction<'_, Postgres>,
    server: i64,
    keep: &[String],
    now: i64,
) -> anyhow::Result<()> {
    sqlx::query("UPDATE server_alert_events SET resolved_at=$3,resolution='disabled' WHERE server_id=$1 AND resolved_at IS NULL AND NOT(source_key=ANY($2))")
        .bind(server).bind(keep).bind(now).execute(&mut **tx).await?;
    sqlx::query("UPDATE notification_outbox o SET status='cancelled' FROM server_alert_events e WHERE o.event_id=e.id AND e.server_id=$1 AND NOT(e.source_key=ANY($2)) AND o.status='pending'")
        .bind(server).bind(keep).execute(&mut **tx).await?;
    Ok(())
}
