use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn chain_is_private_billed_once_and_requires_both_applied_endpoints(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let a = panel.create_server(&cookie, "Entry server").await?;
    let b = panel.create_server(&cookie, "Exit server").await?;
    let entry = id(&panel.create_node(&cookie, a, "Entry").await?)?;
    let exit = id(&panel.create_node(&cookie, b, "Exit").await?)?;
    let chain = call(
        &panel,
        &cookie,
        Method::POST,
        "/chains",
        Some(json!({"name":"Two hops","entry_node_id":entry,"exit_node_id":exit})),
    )
    .await?;
    let chain_id = id(&chain)?;
    let relay: Uuid = sqlx::query_scalar("SELECT relay_uuid FROM singbox_chains WHERE id=$1")
        .bind(chain_id)
        .fetch_one(&pool)
        .await?;
    assert!(!chain.to_string().contains(&relay.to_string()));
    let user = panel.create_user(&cookie, "Member").await?;
    let uid = id(&user)?;
    let group = create_policy(&panel, &cookie, &[], &[chain_id]).await?;
    set_policies(&panel, &cookie, uid, &[group]).await?;
    assert_eq!(eligible(&pool, uid, now_timestamp()).await?, vec![entry]);
    let direct = panel
        .admin(
            Method::POST,
            &format!("{ROOT}/users/{uid}/accesses"),
            &cookie,
            Some(json!({"node_id":entry})),
        )
        .await?;
    assert_eq!(direct.status(), StatusCode::CONFLICT);
    entitlements::refresh(&pool, now_timestamp()).await?;
    panel.publish_now().await?;
    let entry_config = latest_config(&pool, a).await?;
    let exit_config = latest_config(&pool, b).await?;
    assert_eq!(
        entry_config["route"]["rules"][0]["outbound"],
        format!("chain-{chain_id}")
    );
    assert_eq!(entry_config["outbounds"][1]["uuid"], relay.to_string());
    assert_eq!(
        exit_config["inbounds"][0]["users"][0]["uuid"],
        relay.to_string()
    );
    assert_eq!(
        entry_config["experimental"]["v2ray_api"]["stats"]["users"],
        json!([format!("u{uid}_n{entry}")])
    );
    assert_eq!(
        exit_config["experimental"]["v2ray_api"]["stats"]["users"],
        json!([])
    );
    let sub = format!(
        "{}?format=singbox",
        user["subscription_url"].as_str().unwrap()
    );
    assert_eq!(
        panel.client.get(&sub).send().await?.status(),
        StatusCode::CONFLICT
    );
    sqlx::query(
        "UPDATE server_module_status SET applied_rev=target_rev,healthy=TRUE WHERE server_id=$1",
    )
    .bind(a)
    .execute(&pool)
    .await?;
    assert_eq!(
        panel.client.get(&sub).send().await?.status(),
        StatusCode::CONFLICT
    );
    applied(&pool).await?;
    let client: Value = panel
        .client
        .get(&sub)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        client["outbounds"][0]["outbounds"],
        json!([format!("node-{entry}")])
    );
    assert!(!client.to_string().contains(&relay.to_string()));
    let list = call(
        &panel,
        &cookie,
        Method::GET,
        &format!("/users/{uid}/accesses"),
        None,
    )
    .await?;
    assert_eq!(list.as_array().unwrap().len(), 1);
    // Losing the exit revokes the entry; it must never fall back to direct.
    call(
        &panel,
        &cookie,
        Method::DELETE,
        &format!("/nodes/{exit}"),
        None,
    )
    .await?;
    assert!(eligible(&pool, uid, now_timestamp()).await?.is_empty());
    assert_eq!(
        panel.client.get(&sub).send().await?.status(),
        StatusCode::CONFLICT
    );
    panel.publish_now().await?;
    assert!(
        latest_config(&pool, a).await?["inbounds"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        call(&panel, &cookie, Method::GET, "/chains", None).await?[0]["available"],
        false
    );
    call(
        &panel,
        &cookie,
        Method::PUT,
        &format!("/policy-groups/{group}"),
        Some(json!({"name":"Empty","node_ids":[],"chain_ids":[]})),
    )
    .await?;
    call(
        &panel,
        &cookie,
        Method::DELETE,
        &format!("/chains/{chain_id}"),
        None,
    )
    .await?;
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn chain_rejects_existing_direct_grants_cycles_nesting_and_same_server(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let a = panel.create_server(&cookie, "A").await?;
    let b = panel.create_server(&cookie, "B").await?;
    let c = panel.create_server(&cookie, "C").await?;
    let na = id(&panel.create_node(&cookie, a, "A").await?)?;
    let na2 = id(&panel.create_node(&cookie, a, "A2").await?)?;
    let nb = id(&panel.create_node(&cookie, b, "B").await?)?;
    let nc = id(&panel.create_node(&cookie, c, "C").await?)?;
    let user = id(&panel.create_user(&cookie, "Direct").await?)?;
    panel.grant(&cookie, user, na2).await?;
    for (entry, exit, status) in [
        (na, na2, StatusCode::BAD_REQUEST),
        (na2, nb, StatusCode::CONFLICT),
        (na, na, StatusCode::BAD_REQUEST),
    ] {
        assert_eq!(
            panel
                .admin(
                    Method::POST,
                    &format!("{ROOT}/chains"),
                    &cookie,
                    Some(json!({"name":"Invalid","entry_node_id":entry,"exit_node_id":exit}))
                )
                .await?
                .status(),
            status
        );
    }
    let chain = id(&call(
        &panel,
        &cookie,
        Method::POST,
        "/chains",
        Some(json!({"name":"AB","entry_node_id":na,"exit_node_id":nb})),
    )
    .await?)?;
    for (entry, exit) in [(nb, na), (nb, nc), (nc, na), (na, nc)] {
        assert_eq!(
            panel
                .admin(
                    Method::POST,
                    &format!("{ROOT}/chains"),
                    &cookie,
                    Some(json!({"name":"Invalid","entry_node_id":entry,"exit_node_id":exit}))
                )
                .await?
                .status(),
            StatusCode::CONFLICT
        );
    }
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &format!("{ROOT}/policy-groups"),
                &cookie,
                Some(json!({"name":"Bypass","node_ids":[na],"chain_ids":[]}))
            )
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    create_policy(&panel, &cookie, &[], &[chain]).await?;
    assert_eq!(
        panel
            .admin(
                Method::DELETE,
                &format!("{ROOT}/chains/{chain}"),
                &cookie,
                None
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn retiring_chain_exit_schedules_entry_revocation_even_without_user_requests(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let a = panel.create_server(&cookie, "A").await?;
    let b = panel.create_server(&cookie, "B").await?;
    let entry = id(&panel.create_node(&cookie, a, "Entry").await?)?;
    let exit = id(&panel.create_node(&cookie, b, "Exit").await?)?;
    let chain = id(&call(
        &panel,
        &cookie,
        Method::POST,
        "/chains",
        Some(json!({"name":"AB","entry_node_id":entry,"exit_node_id":exit})),
    )
    .await?)?;
    let user = id(&panel.create_user(&cookie, "Member").await?)?;
    let group = create_policy(&panel, &cookie, &[], &[chain]).await?;
    set_policies(&panel, &cookie, user, &[group]).await?;
    entitlements::refresh(&pool, now_timestamp()).await?;
    panel.publish_now().await?;
    // Simulate the core's soft-delete boundary; no plugin API or traffic occurs.
    sqlx::query("UPDATE servers SET deleted_at=$2 WHERE id=$1")
        .bind(b)
        .bind(now_timestamp())
        .execute(&pool)
        .await?;
    entitlements::refresh(&pool, now_timestamp()).await?;
    let dirty: Option<i64> = sqlx::query_scalar("SELECT dirty_at FROM servers WHERE id=$1")
        .bind(a)
        .fetch_one(&pool)
        .await?;
    assert!(dirty.is_some());
    panel.publish_now().await?;
    assert!(
        latest_config(&pool, a).await?["inbounds"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    Ok(())
}
