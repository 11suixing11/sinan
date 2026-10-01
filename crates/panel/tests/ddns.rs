#![forbid(unsafe_code)]
mod business_support;
#[path = "../../protocol/tests/support/release.rs"]
mod release_support;

use anyhow::Result;
use business_support::TestPanel;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;

const TOKEN: &str = "TEST_ONLY_CLOUDFLARE_TOKEN";

fn input(server: i64) -> Value {
    json!({"config":{"name":"测试解析","server_id":server,"zone_id":"00000000000000000000000000000001","record_name":"Node.EXAMPLE.com.","record_type":"A","ttl":300,"proxied":false,"interval_secs":300,"enabled":false},"api_token":TOKEN})
}

#[sqlx::test]
async fn administrator_crud_redacts_credentials_guards_revisions_and_never_deletes_remote_dns(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let server = panel.create_server(&cookie, "TEST_ONLY DNS").await?;
    let body = input(server);
    assert_eq!(
        panel
            .admin(
                Method::POST,
                "/api/plugins/ddns/rules",
                &cookie,
                Some(body.clone())
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    panel
        .admin(
            Method::POST,
            &format!("/api/plugins/ddns/servers/{server}/enable"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?;
    for method in [Method::GET, Method::POST] {
        assert_eq!(
            panel
                .client
                .request(method, format!("{}/api/plugins/ddns/rules", panel.base))
                .json(&body)
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let response = panel
        .admin(
            Method::POST,
            "/api/plugins/ddns/rules",
            &cookie,
            Some(body.clone()),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CREATED);
    let rule: Value = response.json().await?;
    assert!(!rule.to_string().contains(TOKEN));
    assert!(rule.get("api_token").is_none());
    assert_eq!(rule["token_configured"], true);
    assert_eq!(rule["config"]["record_name"], "node.example.com");
    let path = format!("/api/plugins/ddns/rules/{}", rule["id"].as_str().unwrap());
    for (method, suffix) in [
        (Method::PATCH, ""),
        (Method::DELETE, ""),
        (Method::POST, "/sync"),
    ] {
        assert_eq!(
            panel
                .client
                .request(method, format!("{}{path}{suffix}", panel.base))
                .json(&body)
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    assert_eq!(
        panel
            .admin(
                Method::POST,
                "/api/plugins/ddns/rules",
                &cookie,
                Some(body.clone())
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let mut edit = json!({"config":rule["config"],"revision":1});
    edit["config"]["proxied"] = true.into();
    let updated: Value = panel
        .admin(Method::PATCH, &path, &cookie, Some(edit.clone()))
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(updated["config"]["ttl"], 1);
    assert_eq!(updated["revision"], 2);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT api_token FROM ddns_rules")
            .fetch_one(&pool)
            .await?,
        TOKEN
    );
    assert_eq!(
        panel
            .admin(Method::PATCH, &path, &cookie, Some(edit.clone()))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    edit["revision"] = 2.into();
    edit["config"]["record_name"] = "other.example.com".into();
    assert_eq!(
        panel
            .admin(Method::PATCH, &path, &cookie, Some(edit))
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        panel
            .admin(Method::POST, &format!("{path}/sync"), &cookie, None)
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let edit =
        json!({"config":updated["config"],"revision":2,"api_token":"TEST_ONLY_REPLACEMENT_TOKEN"});
    let response: Value = panel
        .admin(Method::PATCH, &path, &cookie, Some(edit))
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(!response.to_string().contains("REPLACEMENT_TOKEN"));
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT api_token FROM ddns_rules")
            .fetch_one(&pool)
            .await?,
        "TEST_ONLY_REPLACEMENT_TOKEN"
    );
    sqlx::query("UPDATE ddns_rules SET lease_until=$1")
        .bind(sinan_protocol::now_timestamp() + 60)
        .execute(&pool)
        .await?;
    assert_eq!(
        panel
            .admin(Method::DELETE, &path, &cookie, None)
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    sqlx::query("UPDATE ddns_rules SET lease_until=0")
        .execute(&pool)
        .await?;
    assert_eq!(
        panel
            .admin(Method::DELETE, &path, &cookie, None)
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    let rules: Vec<Value> = panel
        .admin(Method::GET, "/api/plugins/ddns/rules", &cookie, None)
        .await?
        .json()
        .await?;
    assert!(rules.is_empty());
    Ok(())
}

#[sqlx::test]
async fn only_new_static_reports_are_fresh_and_missing_public_ip_never_contacts_provider(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let server = panel.create_server(&cookie, "TEST_ONLY IP").await?;
    panel
        .admin(
            Method::POST,
            &format!("/api/plugins/ddns/servers/{server}/enable"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?;
    let mut body = input(server);
    body["config"]["enabled"] = true.into();
    let rule: Value = panel
        .admin(Method::POST, "/api/plugins/ddns/rules", &cookie, Some(body))
        .await?
        .error_for_status()?
        .json()
        .await?;
    let path = format!(
        "/api/plugins/ddns/rules/{}/sync",
        rule["id"].as_str().unwrap()
    );
    // No background worker runs in this fixture. This manual sync must stop
    // locally, even though the rule contains a syntactically valid test token.
    let response: Value = panel
        .admin(Method::POST, &path, &cookie, None)
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(response["error_code"], "server_offline");
    assert!(response["ip_received_at"].is_null());
    let info: sinan_protocol::StaticInfo = serde_json::from_value(
        json!({"hostname":"test","os":"test","arch":"x86_64","cpu_model":"test","cpu_cores":1,"memory_total":1,"ip_addresses":["192.0.2.1"]}),
    )?;
    sinan_panel::agent_api::process_message(
        &panel.state,
        server,
        sinan_protocol::Message::TelemetryStatic(info.clone()),
    )
    .await?;
    let at: Option<i64> =
        sqlx::query_scalar("SELECT static_info_received_at FROM servers WHERE id=$1")
            .bind(server)
            .fetch_one(&pool)
            .await?;
    assert!(at.is_some());
    sqlx::query("UPDATE servers SET static_info_received_at=NULL,deleted_at=1 WHERE id=$1")
        .bind(server)
        .execute(&pool)
        .await?;
    sinan_panel::agent_api::process_message(
        &panel.state,
        server,
        sinan_protocol::Message::TelemetryStatic(info),
    )
    .await?;
    assert!(
        sqlx::query_scalar::<_, Option<i64>>(
            "SELECT static_info_received_at FROM servers WHERE id=$1"
        )
        .bind(server)
        .fetch_one(&pool)
        .await?
        .is_none()
    );
    Ok(())
}
