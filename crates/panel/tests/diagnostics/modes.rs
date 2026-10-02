use super::*;
use uuid::Uuid;

#[sqlx::test(migrations = "./migrations")]
async fn modes_require_admin_confirmation_gate_capability_and_bound_daily_targets(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, _ack) = panel.authenticated_device(&cookie, "入口夹具").await?;
    capable(&panel, server).await?;
    fixture(&panel).await?;
    let path = format!("/api/servers/{server}/node-quality/reports");
    for request in [
        json!({}),
        json!({"confirm_full":false}),
        json!({"mode":"daily","network_mode":"normal"}),
        json!({"mode":"daily","upload_report":true}),
    ] {
        assert_eq!(
            panel
                .admin(Method::POST, &path, &cookie, Some(request))
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    sqlx::query(
        "UPDATE servers SET capabilities=capabilities-'diagnostic:nodequality-modes' WHERE id=$1",
    )
    .bind(server)
    .execute(&panel.state.pool)
    .await?;
    assert_eq!(
        panel
            .admin(Method::POST, &path, &cookie, Some(json!({"mode":"daily"})))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    capable(&panel, server).await?;
    sqlx::query(
        "UPDATE servers SET static_info=static_info || '{\"os\":\"freebsd\"}'::jsonb WHERE id=$1",
    )
    .bind(server)
    .execute(&panel.state.pool)
    .await?;
    let view: Value = panel
        .admin(
            Method::GET,
            &format!("/api/servers/{server}/node-quality"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(view["plugin_ready"], false);
    assert_eq!(view["proxy_activity"]["state"], "not_enabled");
    assert!(view["plugin_reason"].as_str().unwrap().contains("Linux"));
    capable(&panel, server).await?;
    for index in 0..8 {
        let spec = sinan_protocol::ProbeSpec {
            id: Uuid::new_v4(),
            name: format!("private target {index}"),
            kind: sinan_protocol::ProbeKind::Tcp,
            target: "127.0.0.1".into(),
            port: Some(443),
            interval_secs: 60,
            carrier: String::new(),
            enabled: index != 0,
        };
        sqlx::query("INSERT INTO network_probes(id,server_id,spec,target_authorization) VALUES($1,$2,$3,$4)")
            .bind(spec.id)
            .bind(server)
            .bind(serde_json::to_value(spec)?)
            .bind(json!({"region":"fixture","source":"TEST_ONLY owned fixture","scope":"owned","evidence":"TEST_ONLY synthetic diagnostic target","expires_at":null}))
            .execute(&panel.state.pool)
            .await?;
    }
    for authorization in [
        Value::Null,
        json!({"region":"fixture","source":"TEST_ONLY expired","scope":"owned","evidence":"TEST_ONLY synthetic target","expires_at":sinan_protocol::now_timestamp()-1}),
    ] {
        let id = Uuid::new_v4();
        sqlx::query("INSERT INTO network_probes(id,server_id,spec,target_authorization) VALUES($1,$2,$3,$4)")
            .bind(id).bind(server).bind(json!({"id":id,"name":"not permitted","kind":"tcp","target":"unpermitted.example.test","port":443,"interval_secs":60,"carrier":"fixture","enabled":true}))
            .bind(authorization).execute(&panel.state.pool).await?;
    }
    let daily: Value = panel
        .admin(
            Method::POST,
            &path,
            &cookie,
            Some(json!({"mode":"daily","ip_version":"ipv6"})),
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM deployments WHERE server_id=$1")
            .bind(server)
            .fetch_one(&panel.state.pool)
            .await?,
        0,
        "reading diagnostic activity must not create proxy deployment evidence"
    );
    assert_eq!(daily["agent_completed"], false);
    assert!(daily["cancel_requested_at"].is_null());
    assert!(daily["cancel_error"].is_null());
    assert_eq!(daily["job"]["version"], diagnostics::PLUGIN_VERSION);
    assert_eq!(daily["job"]["timeout_secs"], 90);
    assert_eq!(daily["job"]["options"]["network_mode"], "low");
    assert_eq!(daily["job"]["options"]["upload_report"], "false");
    let targets: Value =
        serde_json::from_str(daily["job"]["options"]["daily_targets"].as_str().unwrap())?;
    assert_eq!(targets.as_array().unwrap().len(), 4);
    assert!(!serde_json::to_string(&targets)?.contains("unpermitted.example.test"));
    assert_eq!(
        daily["expected_sections"],
        json!(["net_quality", "environment"])
    );
    assert_eq!(
        panel
            .admin(Method::POST, &path, &cookie, Some(json!({"mode":"daily"})))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    sqlx::query(
        "UPDATE diagnostic_jobs SET status='cancel_requested',cancel_requested_at=$2 WHERE id=$1",
    )
    .bind(Uuid::parse_str(daily["id"].as_str().unwrap())?)
    .bind(sinan_protocol::now_timestamp())
    .execute(&panel.state.pool)
    .await?;
    for request in [
        json!({"mode":"daily"}),
        json!({"mode":"full","confirm_full":true,"acknowledge_traffic_warning":true}),
    ] {
        assert_eq!(
            panel
                .admin(Method::POST, &path, &cookie, Some(request))
                .await?
                .status(),
            StatusCode::CONFLICT
        );
    }
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn full_stays_disabled_after_admin_acknowledges_unknown_traffic(pool: PgPool) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, _ack) = panel.authenticated_device(&cookie, "流量未知夹具").await?;
    capable(&panel, server).await?;
    fixture(&panel).await?;
    sqlx::query("INSERT INTO deployments(server_id,module,rev,bundle,bundle_sha256,created_at) VALUES($1,'singbox',1,'TEST_ONLY','TEST_ONLY',$2)").bind(server).bind(sinan_protocol::now_timestamp()).execute(&panel.state.pool).await?;
    let path = format!("/api/servers/{server}/node-quality/reports");
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &path,
                &cookie,
                Some(json!({"confirm_full":true}))
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let response = panel
        .admin(
            Method::POST,
            &path,
            &cookie,
            Some(json!({"confirm_full":true,"acknowledge_traffic_warning":true})),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(response.text().await?.contains("离线受控工具链"));
    let view: Value = panel
        .admin(
            Method::GET,
            &format!("/api/servers/{server}/node-quality"),
            &cookie,
            None,
        )
        .await?
        .json()
        .await?;
    assert_eq!(view["proxy_activity"]["state"], "unknown");
    assert_eq!(view["full_ready"], false);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM diagnostic_jobs WHERE server_id=$1")
            .bind(server)
            .fetch_one(&panel.state.pool)
            .await?,
        0
    );
    Ok(())
}

#[sqlx::test(migrations = "./migrations")]
async fn recent_proxy_traffic_remains_a_warning_even_when_metrics_show_idle(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, _ack) = panel.authenticated_device(&cookie, "持续流量夹具").await?;
    capable(&panel, server).await?;
    fixture(&panel).await?;
    let node = panel.create_node(&cookie, server, "计量节点").await?;
    let account = panel.create_user(&cookie, "计量账户").await?;
    let epoch = Uuid::new_v4();
    let now = sinan_protocol::now_timestamp();
    sqlx::query(
        "INSERT INTO usage_batches(server_id,epoch,seq,payload_hash) VALUES($1,$2,1,'TEST_ONLY')",
    )
    .bind(server)
    .bind(epoch)
    .execute(&panel.state.pool)
    .await?;
    sqlx::query("INSERT INTO usage_records(server_id,epoch,seq,stat_name,user_id,node_id,uplink,downlink,period_start,period_end) VALUES($1,$2,1,'TEST_ONLY',$3,$4,1024,1024,$5,$6)")
        .bind(server).bind(epoch).bind(account["id"].as_i64().unwrap()).bind(node["id"].as_i64().unwrap())
        .bind(now-30).bind(now).execute(&panel.state.pool).await?;
    sqlx::query("UPDATE servers SET latest_metrics='{\"net_in_speed\":0,\"net_out_speed\":0}'::jsonb WHERE id=$1")
        .bind(server).execute(&panel.state.pool).await?;
    let view: Value = panel
        .admin(
            Method::GET,
            &format!("/api/servers/{server}/node-quality"),
            &cookie,
            None,
        )
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(view["proxy_activity"]["state"], "active");
    let path = format!("/api/servers/{server}/node-quality/reports");
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &path,
                &cookie,
                Some(json!({"confirm_full":true}))
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let response = panel
        .admin(
            Method::POST,
            &path,
            &cookie,
            Some(json!({"confirm_full":true,"acknowledge_traffic_warning":true})),
        )
        .await?;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(response.text().await?.contains("离线受控工具链"));
    assert_eq!(view["proxy_activity"]["last_positive_at"], now);
    Ok(())
}
