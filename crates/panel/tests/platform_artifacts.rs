#![forbid(unsafe_code)]

mod business_support;

use anyhow::Result;
use business_support::{TestPanel, id};
use reqwest::StatusCode;
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[sqlx::test]
async fn runtime_selection_matches_abi_and_preserves_legacy_devices(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel.authenticated_device(&cookie, "abi-test").await?;
    let node = id(&panel.create_node(&cookie, server, "abi-node").await?)?;
    let user = id(&panel.create_user(&cookie, "abi-user").await?)?;
    panel.grant(&cookie, user, node).await?;
    panel.publish_now().await?;
    let root = panel
        .state
        .config
        .data_dir
        .join("artifacts/sing-box/1.14.2");
    std::fs::create_dir_all(&root)?;
    let targets = [
        "amd64",
        "linux-gnu-amd64",
        "linux-musl-amd64",
        "freebsd-amd64",
    ];
    let mut sums = String::new();
    for target in targets {
        std::fs::write(root.join(target), target.as_bytes())?;
        sums.push_str(&format!(
            "{:x}  {target}\n",
            Sha256::digest(target.as_bytes())
        ));
    }
    std::fs::write(root.join("SHA256SUMS"), sums)?;
    for (info, expected) in [
        (json!({"arch":"amd64"}), Some("amd64")),
        (
            json!({"arch":"amd64","os":"linux","libc":"gnu"}),
            Some("linux-gnu-amd64"),
        ),
        (
            json!({"arch":"amd64","os":"linux","libc":"musl"}),
            Some("linux-musl-amd64"),
        ),
        (
            json!({"arch":"amd64","os":"freebsd"}),
            Some("freebsd-amd64"),
        ),
        (json!({"arch":"amd64","os":"linux","libc":"unknown"}), None),
        (json!({"arch":"amd64","os":"windows"}), None),
    ] {
        sqlx::query("UPDATE servers SET static_info=$2 WHERE id=$1")
            .bind(server)
            .bind(info)
            .execute(&panel.state.pool)
            .await?;
        let response = panel
            .client
            .get(format!("{}/api/agent/v1/manifest", panel.base))
            .bearer_auth(&ack.session_token)
            .send()
            .await?;
        if let Some(target) = expected {
            assert_eq!(response.status(), StatusCode::OK);
            let value: serde_json::Value = response.json().await?;
            assert!(
                value["modules"]["singbox"]["artifact"]["url"]
                    .as_str()
                    .unwrap()
                    .ends_with(target)
            );
        } else {
            assert!(!response.status().is_success());
        }
    }
    std::fs::remove_file(root.join("linux-gnu-amd64"))?;
    for (libc, expected) in [("gnu", StatusCode::OK), ("musl", StatusCode::NOT_FOUND)] {
        if libc == "musl" {
            std::fs::remove_file(root.join("linux-musl-amd64"))?;
        }
        sqlx::query("UPDATE servers SET static_info=$2 WHERE id=$1")
            .bind(server)
            .bind(json!({"arch":"amd64","os":"linux","libc":libc}))
            .execute(&panel.state.pool)
            .await?;
        let response = panel
            .client
            .get(format!("{}/api/agent/v1/manifest", panel.base))
            .bearer_auth(&ack.session_token)
            .send()
            .await?;
        assert_eq!(response.status(), expected);
    }
    Ok(())
}
