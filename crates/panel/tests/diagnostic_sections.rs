#![forbid(unsafe_code)]

mod business_support;
mod release_fixture;
#[path = "../../protocol/tests/support/release.rs"]
mod release_support;

use anyhow::Result;
use business_support::TestPanel;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sinan_panel::{agent_api, diagnostics};
use sinan_protocol::{Hello, HelloAck, Message, PROTOCOL_VERSION};
use sqlx::PgPool;
use std::collections::BTreeMap;

async fn capable(panel: &TestPanel, server_id: i64) -> Result<()> {
    agent_api::process_message(
        &panel.state,
        server_id,
        Message::Hello(Hello {
            agent_version: "diagnostic-test".into(),
            protocol_version: PROTOCOL_VERSION,
            capabilities: vec![
                "diagnostic:nodequality".into(),
                sinan_protocol::DIAGNOSTIC_SECTIONS_CAPABILITY.into(),
                sinan_protocol::release::ARTIFACT_SIGNATURE_CAPABILITY.into(),
            ],
            applied: BTreeMap::new(),
        }),
    )
    .await
}

async fn fixture(panel: &TestPanel) -> Result<()> {
    let binary = b"fixed diagnostic fixture";
    let archive = release_fixture::archive("nodequality", binary)?;
    release_fixture::write(
        &panel.state.config.data_dir,
        "nodequality",
        diagnostics::PLUGIN_VERSION,
        "nodequality",
        &archive,
        binary,
        "tar.gz",
    )?;
    Ok(())
}

async fn update(
    panel: &TestPanel,
    ack: &HelloAck,
    id: &str,
    payload: Value,
) -> Result<reqwest::Response> {
    Ok(panel
        .client
        .post(format!("{}/api/agent/v1/diagnostics/{id}", panel.base))
        .bearer_auth(&ack.session_token)
        .json(&payload)
        .send()
        .await?)
}

#[sqlx::test(migrations = "./migrations")]
async fn diagnostic_chapters_survive_failure_duplicates_late_delivery_and_recreation(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server_id, _socket, ack) = panel.authenticated_device(&cookie, "章节设备").await?;
    let (_other, _other_socket, other_ack) =
        panel.authenticated_device(&cookie, "其他设备").await?;
    capable(&panel, server_id).await?;
    fixture(&panel).await?;
    let record: Value = panel
        .admin(
            Method::POST,
            &format!("/api/servers/{server_id}/node-quality/reports"),
            &cookie,
            Some(json!({})),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    let id = record["id"].as_str().unwrap();
    let endpoint = format!("{}/api/agent/v1/diagnostics/{id}/sections", panel.base);
    let chapter = json!({"id":id,"name":"header_info","text":"已经完成的报告信息","complete":true,"revision":2,"collected_at":sinan_protocol::now_timestamp()});
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&other_ack.session_token)
            .json(&chapter)
            .send()
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    for _ in 0..2 {
        assert_eq!(
            panel
                .client
                .post(&endpoint)
                .bearer_auth(&ack.session_token)
                .json(&chapter)
                .send()
                .await?
                .status(),
            StatusCode::NO_CONTENT
        );
    }
    let mut collision = chapter.clone();
    collision["text"] = json!("different content");
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&collision)
            .send()
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    collision["revision"] = json!(1);
    collision["complete"] = json!(false);
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&collision)
            .send()
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        update(
            &panel,
            &ack,
            id,
            json!({"id":id,"status":"failed","error":"OOM fixture"})
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    let partial = json!({"id":id,"name":"hardware_quality","text":"停止前的硬件输出","complete":false,"revision":1,"collected_at":sinan_protocol::now_timestamp()});
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&partial)
            .send()
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    let view: Value = panel
        .admin(
            Method::GET,
            &format!("/api/servers/{server_id}/node-quality"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    let report = &view["reports"][0];
    assert_eq!(report["status"], "failed");
    assert_eq!(report["report_completeness"], "partial");
    assert_eq!(report["sections"].as_array().unwrap().len(), 2);
    assert_eq!(report["sections"][0]["text"], chapter["text"]);
    assert_eq!(report["report"], Value::Null);
    assert_eq!(report["error"], "OOM fixture");
    let recreated =
        sinan_panel::AppState::new(panel.state.pool.clone(), (*panel.state.config).clone()).await?;
    let saved: String = sqlx::query_scalar(
        "SELECT text FROM diagnostic_report_sections WHERE job_id=$1 AND name='header_info'",
    )
    .bind(uuid::Uuid::parse_str(id)?)
    .fetch_one(&recreated.pool)
    .await?;
    assert_eq!(saved, "已经完成的报告信息");
    for name in [
        "hardware_quality",
        "ip_quality",
        "net_quality",
        "backroute_trace",
    ] {
        let final_chapter = json!({"id":id,"name":name,"text":format!("saved {name}"),"complete":true,"revision":3,"collected_at":sinan_protocol::now_timestamp()});
        assert_eq!(
            panel
                .client
                .post(&endpoint)
                .bearer_auth(&ack.session_token)
                .json(&final_chapter)
                .send()
                .await?
                .status(),
            StatusCode::NO_CONTENT
        );
    }
    let state: (String, String) =
        sqlx::query_as("SELECT status,report_completeness FROM diagnostic_jobs WHERE id=$1")
            .bind(uuid::Uuid::parse_str(id)?)
            .fetch_one(&recreated.pool)
            .await?;
    assert_eq!(state, ("failed".into(), "complete".into()));
    let mut invalid = chapter;
    invalid["name"] = json!("undeclared");
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&invalid)
            .send()
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    invalid["name"] = json!("header_info");
    invalid["text"] = json!("a".repeat(65537));
    assert_eq!(
        panel
            .client
            .post(&endpoint)
            .bearer_auth(&ack.session_token)
            .json(&invalid)
            .send()
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn legacy_report_text_is_retained_with_unknown_chapter_completeness(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server_id, _socket, ack) = panel.authenticated_device(&cookie, "历史设备").await?;
    let id = uuid::Uuid::new_v4();
    let now = sinan_protocol::now_timestamp();
    sqlx::query("INSERT INTO diagnostic_jobs(id,server_id,job,created_at,updated_at,expires_at) VALUES($1,$2,'{}',$3,$3,$4)").bind(id).bind(server_id).bind(now).bind(now+300).execute(&panel.state.pool).await?;
    assert_eq!(
        update(
            &panel,
            &ack,
            &id.to_string(),
            json!({"id":id,"status":"succeeded","report":{"text":"旧版本完整文本"}})
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    let view: Value = panel
        .admin(
            Method::GET,
            &format!("/api/servers/{server_id}/node-quality"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(view["reports"][0]["report"]["text"], "旧版本完整文本");
    assert_eq!(view["reports"][0]["report_completeness"], "legacy");
    assert_eq!(view["reports"][0]["sections"], json!([]));
    Ok(())
}
