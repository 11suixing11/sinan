use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn policy_union_preserves_credentials_until_the_last_source_is_removed(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let server = panel.create_server(&cookie, "Server").await?;
    let n1 = id(&panel.create_node(&cookie, server, "One").await?)?;
    let n2 = id(&panel.create_node(&cookie, server, "Two").await?)?;
    let user = id(&panel.create_user(&cookie, "Member").await?)?;
    let other = id(&panel.create_user(&cookie, "Other").await?)?;
    let direct = panel.grant(&cookie, user, n1).await?;
    let first = create_policy(&panel, &cookie, &[n1, n2, n2], &[]).await?;
    let second = create_policy(&panel, &cookie, &[n2], &[]).await?;
    set_policies(&panel, &cookie, user, &[first, second, second]).await?;
    let before = call(
        &panel,
        &cookie,
        Method::GET,
        &format!("/users/{user}/accesses"),
        None,
    )
    .await?;
    assert_eq!(before.as_array().unwrap().len(), 2);
    assert_eq!(before[0]["uuid"], direct["uuid"]);
    assert_eq!(before[0]["direct_grant"], true);
    assert_eq!(before[1]["direct_grant"], false);
    assert_eq!(
        eligible(&pool, other, now_timestamp()).await?,
        Vec::<i64>::new()
    );
    set_policies(&panel, &cookie, user, &[second]).await?;
    let retained = call(
        &panel,
        &cookie,
        Method::GET,
        &format!("/users/{user}/accesses"),
        None,
    )
    .await?;
    assert_eq!(retained, before);
    call(
        &panel,
        &cookie,
        Method::DELETE,
        &format!("/users/{user}/accesses/{n1}"),
        None,
    )
    .await?;
    assert_eq!(eligible(&pool, user, now_timestamp()).await?, vec![n2]);
    let blocked = panel
        .admin(
            Method::DELETE,
            &format!("{ROOT}/policy-groups/{second}"),
            &cookie,
            None,
        )
        .await?;
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
    let invalid = panel
        .admin(
            Method::PUT,
            &format!("{ROOT}/users/{user}/policy-groups"),
            &cookie,
            Some(json!({"group_ids":[999999]})),
        )
        .await?;
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(eligible(&pool, user, now_timestamp()).await?, vec![n2]);
    set_policies(&panel, &cookie, user, &[]).await?;
    assert!(eligible(&pool, user, now_timestamp()).await?.is_empty());
    set_policies(&panel, &cookie, user, &[second]).await?;
    let rotated = call(
        &panel,
        &cookie,
        Method::GET,
        &format!("/users/{user}/accesses"),
        None,
    )
    .await?;
    assert_ne!(rotated[0]["uuid"], before[1]["uuid"]);
    call(
        &panel,
        &cookie,
        Method::PUT,
        &format!("/policy-groups/{second}"),
        Some(json!({"name":"Changed","node_ids":[n1],"chain_ids":[]})),
    )
    .await?;
    assert_eq!(eligible(&pool, user, now_timestamp()).await?, vec![n1]);
    call(
        &panel,
        &cookie,
        Method::DELETE,
        &format!("/users/{user}"),
        None,
    )
    .await?;
    call(
        &panel,
        &cookie,
        Method::DELETE,
        &format!("/policy-groups/{second}"),
        None,
    )
    .await?;
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_group_and_direct_grants_share_one_stable_credential(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let server = panel.create_server(&cookie, "Server").await?;
    let node = id(&panel.create_node(&cookie, server, "Node").await?)?;
    let user = id(&panel.create_user(&cookie, "Member").await?)?;
    let group = create_policy(&panel, &cookie, &[node], &[]).await?;
    let groups = [group];
    let (direct, grouped) = tokio::join!(
        panel.grant(&cookie, user, node),
        set_policies(&panel, &cookie, user, &groups)
    );
    let direct = direct?;
    grouped?;
    assert_eq!(eligible(&pool, user, now_timestamp()).await?, vec![node]);
    call(
        &panel,
        &cookie,
        Method::DELETE,
        &format!("/users/{user}/accesses/{node}"),
        None,
    )
    .await?;
    let current = call(
        &panel,
        &cookie,
        Method::GET,
        &format!("/users/{user}/accesses"),
        None,
    )
    .await?;
    assert_eq!(current[0]["uuid"], direct["uuid"]);
    assert_eq!(current[0]["direct_grant"], false);
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn new_management_routes_require_administrator_and_validate_inputs(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    for path in [
        "/policy-groups",
        "/package-groups",
        "/chains",
        "/users/1/entitlement",
        "/users/1/policy-groups",
    ] {
        assert_eq!(
            panel
                .client
                .get(format!("{}{ROOT}{path}", panel.base))
                .send()
                .await?
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
    let cookie = panel.admin_cookie().await?;
    for body in [
        json!({"name":"bad","node_ids":[0],"chain_ids":[]}),
        json!({"name":"bad","node_ids":[99999],"chain_ids":[]}),
    ] {
        assert_eq!(
            panel
                .admin(
                    Method::POST,
                    &format!("{ROOT}/policy-groups"),
                    &cookie,
                    Some(body)
                )
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    for (key, value) in [
        ("reset_day", json!(32)),
        ("reset_hour", json!(24)),
        ("timezone", json!("Not/AZone")),
        ("monthly_bytes", json!("18446744073709551616")),
        ("duration_days", json!(0)),
    ] {
        let mut body = plan_body(Some("100"));
        body[key] = value;
        assert_eq!(
            panel
                .admin(
                    Method::POST,
                    &format!("{ROOT}/package-groups"),
                    &cookie,
                    Some(body)
                )
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    Ok(())
}

#[sqlx::test(migrations = false)]
async fn group_migration_retains_legacy_identifiers_credentials_and_usage(
    pool: PgPool,
) -> Result<()> {
    use sqlx::migrate::Migrator;
    use std::borrow::Cow;
    let all = sqlx::migrate!();
    let old = Migrator {
        migrations: Cow::Owned(all.iter().filter(|m| m.version < 16).cloned().collect()),
        ..Migrator::DEFAULT
    };
    old.run(&pool).await?;
    sqlx::raw_sql("INSERT INTO servers(id,name) VALUES(1,'Legacy'); INSERT INTO nodes(id,name,server_id,port,public_host,sni,private_key,public_key,short_id) VALUES(1,'Legacy',1,443,'proxy.example.com','www.example.com','old-key','old-key','1234abcd'); INSERT INTO users(id,name,subscription_token) VALUES(1,'Legacy','TEST_ONLY-token'); INSERT INTO accesses(user_id,node_id,uuid,stat_name) VALUES(1,1,'00000000-0000-0000-0000-000000000001','u1_n1'); INSERT INTO usage_batches VALUES(1,'00000000-0000-0000-0000-000000000002',1,'TEST_ONLY-hash'); INSERT INTO usage_records VALUES(1,'00000000-0000-0000-0000-000000000002',1,'u1_n1',1,1,20,30,100,101);").execute(&pool).await?;
    all.run(&pool).await?;
    let token: String = sqlx::query_scalar("SELECT subscription_token FROM users WHERE id=1")
        .fetch_one(&pool)
        .await?;
    assert_eq!(token, "TEST_ONLY-token");
    let uuid: Uuid = sqlx::query_scalar("SELECT uuid FROM accesses WHERE user_id=1")
        .fetch_one(&pool)
        .await?;
    assert_eq!(uuid, Uuid::from_u128(1));
    assert_eq!(eligible(&pool, 1, now_timestamp()).await?, vec![1]);
    let status: String =
        sqlx::query_scalar("SELECT status FROM singbox_entitlements($1) WHERE user_id=1")
            .bind(now_timestamp())
            .fetch_one(&pool)
            .await?;
    assert_eq!(status, "unmetered");
    let total: String = sqlx::query_scalar("SELECT SUM(uplink+downlink)::text FROM usage_records")
        .fetch_one(&pool)
        .await?;
    assert_eq!(total, "50");
    Ok(())
}
