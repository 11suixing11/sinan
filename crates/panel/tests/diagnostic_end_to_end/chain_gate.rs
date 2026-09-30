use super::*;

#[sqlx::test(migrations = "./migrations")]
async fn saved_preparing_full_is_refused_after_upgrade_without_starting_a_service(
    pool: PgPool,
) -> Result<()> {
    let harness = Harness::start(pool).await?;
    write_artifact(&harness)?;
    let server = harness
        .api(
            Method::POST,
            "/api/servers",
            json!({"name":"已领取任务门禁夹具"}),
        )
        .await?;
    let server_id = server["id"].as_i64().context("server id")?;
    let enrollment = harness
        .api(
            Method::POST,
            &format!("/api/servers/{server_id}/enrollment"),
            json!({}),
        )
        .await?;
    let config = harness.agent_config();
    identity::enroll(
        &config,
        enrollment["token"].as_str().context("enrollment token")?,
    )
    .await?;
    let artifact =
        sinan_panel::artifacts::descriptor(&harness.state, "nodequality", VERSION, "amd64").await?;
    let now = sinan_protocol::now_timestamp();
    let job = sinan_protocol::DiagnosticJob {
        id: uuid::Uuid::new_v4(),
        plugin: "nodequality".into(),
        version: VERSION.into(),
        artifact,
        timeout_secs: 1800,
        expires_at: Some(now + 2100),
        resource_budget: None,
        options: BTreeMap::new(),
    };
    sqlx::query("INSERT INTO diagnostic_jobs(id,server_id,job,status,created_at,updated_at,expires_at) VALUES($1,$2,$3,'queued',$4,$4,$5)")
        .bind(job.id).bind(server_id).bind(serde_json::to_value(&job)?).bind(now)
        .bind(now + 2100).execute(&harness.state.pool).await?;
    // A legacy Agent had already received this HTTP response before rollout.
    // The upgraded Agent must reject its durable Preparing state as well.
    State::open(&config.state_db)?.set_json("diagnostics:active", &json!({"Preparing": job}))?;
    let services = Arc::new(IndependentServices::default());
    let agent = tokio::spawn(transport::run_with_diagnostics(
        config.clone(),
        vec![],
        vec![Arc::new(NodeQualityAdapter::new())],
        Arc::new(FakeResourceOps::new(Arc::new(SystemOps))),
        services.clone(),
        "full-gate-upgrade-fixture",
    ));
    eventually(
        "preparing task refused and acknowledged after upgrade",
        20,
        || async {
            anyhow::ensure!(
                !agent.is_finished(),
                "Agent exited during gate acknowledgement"
            );
            let row: (String, Option<String>, bool) = sqlx::query_as(
                "SELECT status,error,agent_completed FROM diagnostic_jobs WHERE id=$1",
            )
            .bind(job.id)
            .fetch_one(&harness.state.pool)
            .await?;
            Ok(row.0 == "failed"
                && row.2
                && row.1.is_some_and(|error| error.contains("离线受控工具链")))
        },
    )
    .await?;
    assert_eq!(services.starts.load(Ordering::SeqCst), 0);
    assert_eq!(services.stops.load(Ordering::SeqCst), 0);
    assert!(
        !config
            .runtime_root
            .join("diagnostics")
            .join(job.id.to_string())
            .exists()
    );
    assert!(
        State::open(&config.state_db)?
            .get_json::<serde_json::Value>("diagnostics:active")?
            .is_none_or(|checkpoint| checkpoint.is_null())
    );
    agent.abort();
    let _ = agent.await;
    Ok(())
}
