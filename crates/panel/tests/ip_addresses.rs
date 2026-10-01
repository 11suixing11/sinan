#![forbid(unsafe_code)]

mod business_support;
#[path = "../../protocol/tests/support/release.rs"]
mod release_support;

use anyhow::Result;
use business_support::TestPanel;
use reqwest::Method;
use serde_json::{Value, json};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn address_view_groups_public_and_private_without_losing_the_legacy_list(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let id = panel.create_server(&cookie, "地址分类夹具").await?;
    // Public DNS constants are classification inputs; every request is loopback.
    for (addresses, public, private) in [
        (
            json!([
                "172.18.0.1",
                "10.0.0.2",
                "8.8.8.8",
                "2606:4700::1111",
                "fd00::1",
                "::ffff:172.17.0.1",
                "192.168.0.1",
                "172.18.0.1",
                "invalid"
            ]),
            json!(["2606:4700::1111", "8.8.8.8"]),
            json!([
                "10.0.0.2",
                "172.18.0.1",
                "192.168.0.1",
                "::ffff:172.17.0.1",
                "fd00::1"
            ]),
        ),
        (
            json!(["172.17.0.1", "fd00::1", "100.64.0.1", "fe80::1"]),
            json!([]),
            json!(["100.64.0.1", "172.17.0.1", "fd00::1", "fe80::1"]),
        ),
        (json!([]), json!([]), json!([])),
    ] {
        sqlx::query("UPDATE servers SET static_info=$2 WHERE id=$1")
            .bind(id)
            .bind(json!({"ip_addresses": addresses}))
            .execute(&panel.state.pool)
            .await?;
        let view: Value = panel
            .admin(
                Method::GET,
                &format!("/api/servers/{id}/ip-quality"),
                &cookie,
                None,
            )
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_eq!(view["public_ip_addresses"], public);
        assert_eq!(view["private_ip_addresses"], private);
        let all: Vec<_> = public
            .as_array()
            .unwrap()
            .iter()
            .chain(private.as_array().unwrap())
            .collect();
        assert_eq!(view["ip_addresses"], json!(all));
        assert_eq!(view["quality"], json!([]));
    }
    Ok(())
}
