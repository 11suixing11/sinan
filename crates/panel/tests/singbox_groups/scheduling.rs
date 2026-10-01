use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_retries_create_one_assignment_and_do_not_extend_expiry(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let user = id(&panel.create_user(&cookie, "Concurrent member").await?)?;
    let plan = create_plan(&panel, &cookie, Some("100")).await?;
    let request = Uuid::new_v4();
    let (left, right) = tokio::join!(
        assign(&panel, &cookie, user, plan, request),
        assign(&panel, &cookie, user, plan, request)
    );
    assert_eq!(left?, right?);
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM singbox_package_assignments WHERE user_id=$1")
            .bind(user)
            .fetch_one(&pool)
            .await?;
    assert_eq!(count, 1);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn expiry_background_refresh_revokes_runtime_and_reassignment_preserves_credentials(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let server = panel.create_server(&cookie, "Expiry server").await?;
    let node = id(&panel.create_node(&cookie, server, "Expiry node").await?)?;
    let user = id(&panel.create_user(&cookie, "Expiry member").await?)?;
    let original = panel.grant(&cookie, user, node).await?;
    let plan = create_plan(&panel, &cookie, None).await?;
    assign(&panel, &cookie, user, plan, Uuid::new_v4()).await?;
    entitlements::refresh(&pool, now_timestamp()).await?;
    panel.publish_now().await?;
    assert_eq!(
        latest_config(&pool, server).await?["inbounds"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    // Advance the persisted fixture to an expired window without waiting a day.
    // No management/subscription HTTP request is made between expiry and publication.
    sqlx::query(
        "UPDATE singbox_package_assignments SET starts_at=$1-100,expires_at=$1 WHERE user_id=$2",
    )
    .bind(now_timestamp())
    .bind(user)
    .execute(&pool)
    .await?;
    entitlements::refresh(&pool, now_timestamp()).await?;
    panel.publish_now().await?;
    assert!(
        latest_config(&pool, server).await?["inbounds"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        entitlement(&panel, &cookie, user).await?["status"],
        "expired"
    );
    assign(&panel, &cookie, user, plan, Uuid::new_v4()).await?;
    entitlements::refresh(&pool, now_timestamp()).await?;
    panel.publish_now().await?;
    let native = latest_config(&pool, server).await?;
    assert_eq!(native["inbounds"][0]["users"][0]["uuid"], original["uuid"]);
    assert_eq!(
        entitlement(&panel, &cookie, user).await?["status"],
        "active"
    );
    Ok(())
}
