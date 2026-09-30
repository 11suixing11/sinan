#![forbid(unsafe_code)]
mod business_support;
use anyhow::Result;
use business_support::TestPanel;
use reqwest::StatusCode;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

#[sqlx::test]
async fn updates_require_opt_in_matching_platform_and_newer_verified_stable_release(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel.authenticated_device(&cookie, "updates").await?;
    let url = format!("{}/api/agent/v1/update", panel.base);
    assert_eq!(
        panel.client.get(&url).send().await?.status(),
        StatusCode::UNAUTHORIZED
    );
    for (version, target, valid) in [
        ("0.9.0", "linux-musl-amd64", true),
        ("0.10.0", "linux-musl-amd64", true),
        ("1.0.0", "linux-gnu-amd64", true),
        ("2.0.0", "linux-musl-amd64", false),
        ("3.0.0-beta", "linux-musl-amd64", true),
    ] {
        let root = panel
            .state
            .config
            .data_dir
            .join("artifacts/agent")
            .join(version);
        std::fs::create_dir_all(&root)?;
        std::fs::write(root.join(target), b"fixture")?;
        let digest = if valid {
            format!("{:x}", Sha256::digest(b"fixture"))
        } else {
            "0".repeat(64)
        };
        std::fs::write(root.join("SHA256SUMS"), format!("{digest}  {target}\n"))?;
    }
    sqlx::query("UPDATE servers SET static_info=$2 WHERE id=$1")
        .bind(server)
        .bind(json!({"os":"linux","arch":"amd64","libc":"musl","agent_version":"0.3.0"}))
        .execute(&panel.state.pool)
        .await?;
    let fetch = || {
        panel
            .client
            .get(&url)
            .bearer_auth(&ack.session_token)
            .send()
    };
    assert!(fetch().await?.json::<Value>().await?.is_null());
    sqlx::query("UPDATE servers SET agent_settings=jsonb_set(agent_settings,'{auto_update}','true') WHERE id=$1")
        .bind(server).execute(&panel.state.pool).await?;
    let release: Value = fetch().await?.json().await?;
    assert_eq!(release["version"], "0.10.0");
    assert!(
        release["artifact"]["url"]
            .as_str()
            .unwrap()
            .ends_with("linux-musl-amd64")
    );
    for info in [
        json!({"os":"linux","arch":"amd64","libc":"musl","agent_version":"0.10.0"}),
        json!({"os":"windows","arch":"arm64","agent_version":"0.3.0"}),
    ] {
        sqlx::query("UPDATE servers SET static_info=$2 WHERE id=$1")
            .bind(server)
            .bind(info)
            .execute(&panel.state.pool)
            .await?;
        assert!(fetch().await?.json::<Value>().await?.is_null());
    }
    Ok(())
}

#[sqlx::test]
async fn native_installers_render_without_legacy_linux_artifacts(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let server = panel.create_server(&cookie, "native-install").await?;
    let enrollment: Value = panel
        .admin(
            reqwest::Method::POST,
            &format!("/api/servers/{server}/enrollment"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(
        enrollment["windows_install_command"]
            .as_str()
            .unwrap()
            .contains("install.ps1")
    );
    assert!(
        enrollment["freebsd_install_command"]
            .as_str()
            .unwrap()
            .starts_with("fetch ")
    );
    let root = panel
        .state
        .config
        .data_dir
        .join("artifacts/agent")
        .join(env!("CARGO_PKG_VERSION"));
    std::fs::create_dir_all(&root)?;
    let digest = format!("{:x}", Sha256::digest(b"fixture"));
    let mut sums = String::new();
    for target in ["macos-arm64", "freebsd-arm64", "windows-arm64"] {
        std::fs::write(root.join(target), b"fixture")?;
        sums.push_str(&format!("{digest}  {target}\n"));
    }
    std::fs::write(root.join("SHA256SUMS"), sums)?;
    for name in ["install.sh", "install.ps1"] {
        let response = panel
            .client
            .get(format!("{}/{name}", panel.base))
            .query(&[("token", enrollment["token"].as_str().unwrap())])
            .send()
            .await?
            .error_for_status()?;
        let script = response.text().await?;
        assert!(!script.contains("@@"));
        assert!(script.contains(&digest));
        if name.ends_with("sh") {
            use std::io::Write;
            let mut parser = std::process::Command::new("sh")
                .args(["-n"])
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            parser.stdin.take().unwrap().write_all(script.as_bytes())?;
            assert!(parser.wait()?.success());
        }
    }
    Ok(())
}
