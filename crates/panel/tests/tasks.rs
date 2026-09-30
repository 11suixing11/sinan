#![forbid(unsafe_code)]
mod business_support;
use anyhow::Result;
use business_support::TestPanel;
use reqwest::{Method, StatusCode};
use serde_json::json;
use sinan_protocol::{
    CommandResult, CommandStatus, ProbeBatch, ProbeKind, ProbeResult, ProbeSpec, RemoteCommand,
    TaskAck, now_timestamp, telemetry::now_millis,
};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test]
async fn commands_are_authenticated_device_scoped_and_terminal_results_immutable(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel.authenticated_device(&cookie, "commands").await?;
    let (_other, _other_socket, other_ack) = panel.authenticated_device(&cookie, "other").await?;
    let url = format!("/api/servers/{server}/commands");
    let body = json!({"command":"printf fixture","timeout_secs":5,"ttl_secs":60});
    assert_eq!(
        panel
            .client
            .post(format!("{}{url}", panel.base))
            .json(&body)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let command: RemoteCommand = panel
        .admin(Method::POST, &url, &cookie, Some(body))
        .await?
        .error_for_status()?
        .json()
        .await?;
    let pending: Vec<RemoteCommand> = panel
        .client
        .get(format!("{}/api/agent/v1/commands", panel.base))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(pending, vec![command.clone()]);
    let result = CommandResult {
        id: command.id,
        status: CommandStatus::Succeeded,
        finished_at: now_timestamp(),
        stdout: "fixture".into(),
        stderr: String::new(),
        timed_out: false,
        truncated: false,
    };
    let endpoint = format!("{}/api/agent/v1/commands/{}", panel.base, command.id);
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&other_ack.session_token)
            .json(&result)
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    for _ in 0..2 {
        let ack: TaskAck = panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&result)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_eq!(ack.ids, vec![command.id]);
    }
    let mut changed = result;
    changed.stdout = "changed".into();
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&changed)
            .send()
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let pending: Vec<RemoteCommand> = panel
        .client
        .get(format!("{}/api/agent/v1/commands", panel.base))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(pending.is_empty());
    Ok(())
}

#[sqlx::test]
async fn probes_preserve_missing_latency_deduplicate_and_acknowledge_deleted_targets(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel.authenticated_device(&cookie, "probes").await?;
    let spec = ProbeSpec {
        id: Uuid::nil(),
        name: "本地测试".into(),
        kind: ProbeKind::Tcp,
        target: "127.0.0.1".into(),
        port: Some(443),
        interval_secs: 10,
        carrier: String::new(),
        enabled: true,
    };
    let spec: ProbeSpec = panel
        .admin(
            Method::POST,
            &format!("/api/servers/{server}/probes"),
            &cookie,
            Some(serde_json::to_value(spec)?),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    let result = ProbeResult {
        id: Uuid::new_v4(),
        probe_id: spec.id,
        sampled_at: now_millis(),
        latency_ms: None,
        loss_percent: 100.0,
        error: None,
    };
    let endpoint = format!("{}/api/agent/v1/probe-results", panel.base);
    for _ in 0..2 {
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&ProbeBatch {
                results: vec![result.clone()],
            })
            .send()
            .await?
            .error_for_status()?;
    }
    let history: Vec<ProbeResult> = panel
        .admin(
            Method::GET,
            &format!("/api/servers/{server}/probe-results"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(history, vec![result.clone()]);
    let mut changed = result;
    changed.loss_percent = 0.0;
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&ProbeBatch {
                results: vec![changed.clone()]
            })
            .send()
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    panel
        .admin(
            Method::DELETE,
            &format!("/api/servers/{server}/probes/{}", spec.id),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?;
    changed.id = Uuid::new_v4();
    let received: TaskAck = panel
        .client
        .post(&endpoint)
        .bearer_auth(&ack.session_token)
        .json(&ProbeBatch {
            results: vec![changed.clone()],
        })
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(received.ids, vec![changed.id]);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM probe_results")
        .fetch_one(&panel.state.pool)
        .await?;
    assert_eq!(count, 1);
    Ok(())
}
