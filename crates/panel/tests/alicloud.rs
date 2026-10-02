#![forbid(unsafe_code)]
mod business_support;
#[path = "../../protocol/tests/support/release.rs"]
mod release_support;
use anyhow::Result;
use business_support::TestPanel;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

const KEY: &str = "TEST_ONLY_ALICLOUD_ID";
const SECRET: &str = "TEST_ONLY_ALICLOUD_SECRET";
#[sqlx::test]
async fn cloud_account_crud_never_returns_credentials_and_enforces_revisions_and_resource_identity(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let account_body = json!({"name":"测试账号","site":"china","enabled":false,"auto_enabled":false,"limit_gb":100,"access_key_id":KEY,"access_key_secret":SECRET});
    let id: Value = panel
        .admin(
            Method::POST,
            "/api/plugins/alicloud/accounts",
            &cookie,
            Some(account_body.clone()),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    let id = id["id"].as_str().unwrap();
    let overview: Value = panel
        .admin(Method::GET, "/api/plugins/alicloud", &cookie, None)
        .await?
        .error_for_status()?
        .json()
        .await?;
    let text = overview.to_string();
    assert!(!text.contains(KEY) && !text.contains(SECRET) && !text.contains("access_key"));
    for method in [Method::GET, Method::POST] {
        let path = if method == Method::GET {
            "/api/plugins/alicloud"
        } else {
            "/api/plugins/alicloud/accounts"
        };
        assert_eq!(
            panel
                .client
                .request(method, format!("{}{path}", panel.base))
                .json(&account_body)
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let mut edit = account_body.clone();
    edit.as_object_mut().unwrap().remove("access_key_id");
    edit.as_object_mut().unwrap().remove("access_key_secret");
    edit["revision"] = 1.into();
    let account_path = format!("/api/plugins/alicloud/accounts/{id}");
    assert_eq!(
        panel
            .admin(Method::PATCH, &account_path, &cookie, Some(edit.clone()))
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT access_key_secret FROM alicloud_accounts")
            .fetch_one(&pool)
            .await?,
        SECRET
    );
    assert_eq!(
        panel
            .admin(Method::PATCH, &account_path, &cookie, Some(edit.clone()))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    edit["revision"] = 2.into();
    edit["access_key_id"] = "TEST_ONLY_REPLACEMENT".into();
    assert_eq!(
        panel
            .admin(Method::PATCH, &account_path, &cookie, Some(edit))
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let resource_body = json!({"account_id":id,"name":"测试 EIP","kind":"eip","region":"cn-hangzhou","cloud_id":"eip-testonly","auto_enabled":false,"cap_mbps":1});
    let resource: Value = panel
        .admin(
            Method::POST,
            "/api/plugins/alicloud/resources",
            &cookie,
            Some(resource_body.clone()),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        panel
            .admin(
                Method::POST,
                "/api/plugins/alicloud/resources",
                &cookie,
                Some(resource_body.clone())
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        panel
            .admin(Method::DELETE, &account_path, &cookie, None)
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let resource_path = format!(
        "/api/plugins/alicloud/resources/{}",
        resource["id"].as_str().unwrap()
    );
    let mut edit = resource_body;
    edit["revision"] = 1.into();
    edit["region"] = "cn-beijing".into();
    assert_eq!(
        panel
            .admin(Method::PATCH, &resource_path, &cookie, Some(edit))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &format!("{resource_path}/refresh"),
                &cookie,
                None
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    // Disabled account rejects preview locally; no test credential reaches a real cloud.
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &format!("{resource_path}/preview"),
                &cookie,
                Some(
                    json!({"revision":1,"target":{"bandwidth_mbps":1,"charge_type":"PayByTraffic"}})
                )
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        panel
            .admin(Method::DELETE, &resource_path, &cookie, None)
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        panel
            .admin(Method::DELETE, &account_path, &cookie, None)
            .await?
            .status(),
        StatusCode::NO_CONTENT
    );
    let stored: (String, String, bool) =
        sqlx::query_as("SELECT access_key_id,access_key_secret,archived FROM alicloud_accounts")
            .fetch_one(&pool)
            .await?;
    assert_eq!(stored, (String::new(), String::new(), true));
    Ok(())
}
#[sqlx::test]
async fn every_cloud_mutation_requires_administrator_session(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let id = Uuid::new_v4();
    for path in [
        format!("accounts/{id}/refresh"),
        format!("resources/{id}/refresh"),
        format!("operations/{id}/confirm"),
        format!("operations/{id}/cancel"),
        format!("operations/{id}/dismiss"),
    ] {
        assert_eq!(
            panel
                .client
                .post(format!("{}/api/plugins/alicloud/{path}", panel.base))
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    Ok(())
}

#[sqlx::test]
async fn manual_refresh_cannot_bypass_provider_rate_limit(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let id = Uuid::new_v4();
    let now = sinan_protocol::now_timestamp();
    sqlx::query("INSERT INTO alicloud_accounts(id,name,access_key_id,access_key_secret,error_code,last_attempt_at,next_run_at) VALUES($1,'TEST_ONLY throttled',$2,$3,'rate_limited',$4,$5)")
        .bind(id).bind(KEY).bind(SECRET).bind(now-61).bind(now+900).execute(&pool).await?;
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &format!("/api/plugins/alicloud/accounts/{id}/refresh"),
                &cookie,
                None
            )
            .await?
            .status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT next_run_at FROM alicloud_accounts WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await?,
        now + 900
    );
    Ok(())
}
