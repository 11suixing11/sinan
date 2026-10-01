#![forbid(unsafe_code)]
mod business_support;
mod probe_support;
#[path = "../../protocol/tests/support/release.rs"]
mod release_support;
use anyhow::Result;
use business_support::TestPanel;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sinan_protocol::ProbeSpec;
use sqlx::PgPool;

fn spec() -> Value {
    probe_support::authorized(
        json!({"id":uuid::Uuid::nil(),"name":"统一线路","kind":"tcp","target":"probe.example.com","port":443,"interval_secs":30,"carrier":"测试线路","enabled":true}),
    )
}

#[sqlx::test]
async fn assignment_defaults_keep_wire_compatibility_and_preserve_measurement_identity(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (first, _socket, ack) = panel.authenticated_device(&cookie, "first").await?;
    let second = panel.create_server(&cookie, "second").await?;
    let input = json!({"spec":spec(),"default_enabled":true,"server_ids":[second,first]});
    assert_eq!(
        panel
            .client
            .post(format!("{}/api/latency-tasks", panel.base))
            .json(&input)
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let task: Value = panel
        .admin(Method::POST, "/api/latency-tasks", &cookie, Some(input))
        .await?
        .error_for_status()?
        .json()
        .await?;
    let path = format!("/api/latency-tasks/{}", task["id"].as_str().unwrap());
    let probes: Vec<ProbeSpec> = panel
        .client
        .get(format!(
            "{}/api/agent/v1/probes?authorization=1",
            panel.base
        ))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(probes.len(), 1);
    let probe_id = probes[0].id;
    assert_ne!(probe_id.to_string(), task["id"].as_str().unwrap());
    let managed = format!("/api/servers/{first}/probes/{probe_id}");
    for (method, body) in [
        (Method::PATCH, Some(json!(probes[0]))),
        (Method::DELETE, None),
    ] {
        assert_eq!(
            panel.admin(method, &managed, &cookie, body).await?.status(),
            StatusCode::CONFLICT
        );
    }
    let admin_probes: Vec<Value> = panel
        .admin(
            Method::GET,
            &format!("/api/servers/{first}/probes"),
            &cookie,
            None,
        )
        .await?
        .json()
        .await?;
    assert_eq!(admin_probes[0]["task_id"], task["id"]);
    let third = panel.create_server(&cookie, "new default").await?;
    let stale = json!({"spec":task["spec"],"default_enabled":true,"server_ids":[first,second],"revision":1});
    assert_eq!(
        panel
            .admin(Method::PATCH, &path, &cookie, Some(stale))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let tasks: Vec<Value> = panel
        .admin(Method::GET, "/api/latency-tasks", &cookie, None)
        .await?
        .json()
        .await?;
    assert_eq!(tasks[0]["server_ids"], json!([first, second, third]));
    let mut update = json!({"spec":tasks[0]["spec"],"default_enabled":false,"server_ids":[first,third],"revision":2});
    update["spec"]["enabled"] = json!(false);
    update["spec"]["interval_secs"] = json!(60);
    panel
        .admin(Method::PATCH, &path, &cookie, Some(update.clone()))
        .await?
        .error_for_status()?;
    let changed: Vec<ProbeSpec> = panel
        .client
        .get(format!(
            "{}/api/agent/v1/probes?authorization=1",
            panel.base
        ))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(changed[0].id, probe_id);
    assert!(!changed[0].enabled);
    assert_eq!(changed[0].interval_secs, 60);
    update["revision"] = json!(3);
    update["spec"]["target"] = json!("different.example.com");
    assert_eq!(
        panel
            .admin(Method::PATCH, &path, &cookie, Some(update))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM network_probes WHERE server_id=$1")
            .bind(second)
            .fetch_one(&panel.state.pool)
            .await?,
        0
    );
    panel
        .admin(Method::DELETE, &path, &cookie, None)
        .await?
        .error_for_status()?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM network_probes")
            .fetch_one(&panel.state.pool)
            .await?,
        0
    );
    // An offline Agent can finish an old task after removal; acknowledge and discard it.
    let sample = json!({"results":[{"id":uuid::Uuid::new_v4(),"probe_id":probe_id,"sampled_at":sinan_protocol::telemetry::now_millis(),"latency_ms":1.0,"loss_percent":0.0,"error":null}]});
    panel
        .client
        .post(format!("{}/api/agent/v1/probe-results", panel.base))
        .bearer_auth(&ack.session_token)
        .json(&sample)
        .send()
        .await?
        .error_for_status()?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM probe_results")
            .fetch_one(&panel.state.pool)
            .await?,
        0
    );
    Ok(())
}

#[sqlx::test]
async fn group_capacity_and_default_assignment_are_atomic(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let first = panel.create_server(&cookie, "empty").await?;
    let full: Value = panel
        .admin(
            Method::POST,
            "/api/servers",
            &cookie,
            Some(json!({"name":"full","probes":vec![spec();32]})),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    let input = json!({"spec":spec(),"default_enabled":false,"server_ids":[first,full["id"].as_i64().unwrap()]});
    assert_eq!(
        panel
            .admin(Method::POST, "/api/latency-tasks", &cookie, Some(input))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM latency_tasks")
            .fetch_one(&panel.state.pool)
            .await?,
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM network_probes WHERE server_id=$1")
            .bind(first)
            .fetch_one(&panel.state.pool)
            .await?,
        0
    );
    panel
        .admin(
            Method::POST,
            "/api/latency-tasks",
            &cookie,
            Some(json!({"spec":spec(),"default_enabled":true,"server_ids":[]})),
        )
        .await?
        .error_for_status()?;
    assert_eq!(
        panel
            .admin(
                Method::POST,
                "/api/servers",
                &cookie,
                Some(json!({"name":"must rollback","probes":vec![spec();32]}))
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM servers")
            .fetch_one(&panel.state.pool)
            .await?,
        2
    );
    // Concurrent group assignment competes for the last remaining slot.
    let local: Value = panel
        .admin(
            Method::POST,
            "/api/servers",
            &cookie,
            Some(json!({"name":"almost full","probes":vec![spec();30]})),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    let body = json!({"spec":spec(),"default_enabled":false,"server_ids":[local["id"]]});
    let (a, b) = tokio::join!(
        panel.admin(
            Method::POST,
            "/api/latency-tasks",
            &cookie,
            Some(body.clone())
        ),
        panel.admin(Method::POST, "/api/latency-tasks", &cookie, Some(body))
    );
    let mut statuses = vec![a?.status().as_u16(), b?.status().as_u16()];
    statuses.sort();
    assert_eq!(statuses, vec![201, 409]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM network_probes WHERE server_id=$1")
            .bind(local["id"].as_i64().unwrap())
            .fetch_one(&panel.state.pool)
            .await?,
        32
    );
    Ok(())
}
