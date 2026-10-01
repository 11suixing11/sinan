use super::*;
use reqwest::{Method, StatusCode};

#[sqlx::test]
async fn administrator_routes_derive_expected_identity_and_explain_history(
    pool: PgPool,
) -> Result<()> {
    let (panel, server) = fixture(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let path = format!("/api/servers/{server}/runtime-control");
    assert_eq!(
        panel
            .client
            .get(format!("{}{path}", panel.base))
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        panel
            .client
            .post(format!("{}{path}/checkpoint", panel.base))
            .json(&json!({"module":MODULE}))
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let wrong = panel
        .admin(
            Method::POST,
            &format!("{path}/checkpoint"),
            &cookie,
            Some(json!({"module":MODULE,"expected":binding_for_client()})),
        )
        .await?;
    assert_eq!(wrong.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let response = panel
        .admin(
            Method::POST,
            &format!("{path}/checkpoint"),
            &cookie,
            Some(json!({"module":MODULE})),
        )
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    let mut request: RuntimeCheckpointRequest =
        serde_json::from_value(response["request"].clone())?;
    assert_eq!(request.expected.revision, 1);
    expire(&panel.state.pool, &mut request).await?;
    runtime_control::record_checkpoint_result(&panel.state, server, success(&request)).await?;
    let view = panel
        .admin(Method::GET, &path, &cookie, None)
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    assert_eq!(view["checkpoint_required"], true);
    assert_eq!(view["requests"][0]["outcome"], "late");
    assert!(view["requests"][0]["received_at"].as_i64().unwrap() >= request.expires_at);
    assert_eq!(view["checkpoints"], json!([]));
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &format!("{path}/barrier"),
                &cookie,
                Some(json!({"module":MODULE,"minimum_revision":1}))
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    let checkpoint = confirmed(&panel.state, server).await?;
    let response = panel
        .admin(
            Method::POST,
            &format!("{path}/barrier"),
            &cookie,
            Some(json!({"module":MODULE,"minimum_revision":1})),
        )
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    let request: RuntimeRecoveryBarrierRequest =
        serde_json::from_value(response["request"].clone())?;
    assert_eq!(request.expected, checkpoint);
    runtime_control::record_barrier_result(&panel.state, server, barrier_success(&request)).await?;
    let view = panel
        .admin(Method::GET, &path, &cookie, None)
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    assert_eq!(view["checkpoints"][0]["current"], true);
    assert_eq!(view["checkpoints"][0]["barrier_current"], true);
    assert_eq!(view["checkpoints"][0]["minimum_revision"], 1);
    assert!(view["checkpoints"][0]["verified_at"].as_i64().is_some());
    let failed = runtime_control::request_barrier(&panel.state, server, MODULE, 1).await?;
    runtime_control::record_barrier_result(
        &panel.state,
        server,
        RuntimeRecoveryBarrierResult {
            request_id: failed.request_id,
            request_digest: failed.digest()?,
            observed: None,
            minimum_revision: None,
            pending_intents_clear: false,
            success: false,
            error: Some("instance changed during barrier inspection".into()),
        },
    )
    .await?;
    let view = panel
        .admin(Method::GET, &path, &cookie, None)
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    assert_eq!(view["checkpoints"][0]["current"], false);
    assert_eq!(view["checkpoints"][0]["barrier_current"], false);
    // History survives a changed target, but its current flags cannot endorse that target.
    publish(&panel.state.pool, server, 2).await?;
    let view = panel
        .admin(Method::GET, &path, &cookie, None)
        .await?
        .error_for_status()?
        .json::<Value>()
        .await?;
    assert_eq!(view["checkpoints"][0]["current"], false);
    assert_eq!(view["checkpoints"][0]["barrier_current"], false);
    assert_eq!(
        panel
            .admin(
                Method::POST,
                &format!("{path}/barrier"),
                &cookie,
                Some(json!({"module":MODULE,"minimum_revision":2}))
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    Ok(())
}

fn binding_for_client() -> Value {
    json!({"deployment_id":Uuid::new_v4(),"module":MODULE,"revision":1,"bundle_sha256":"a".repeat(64),"binding_digest":"b".repeat(64)})
}

#[sqlx::test]
async fn stored_binding_is_stable_and_mutated_same_revision_is_rejected(
    pool: PgPool,
) -> Result<()> {
    let (panel, server) = fixture(pool).await?;
    let first = runtime_control::request_checkpoint(&panel.state, server, MODULE).await?;
    let second = runtime_control::request_checkpoint(&panel.state, server, MODULE).await?;
    assert_eq!(first, second);
    assert_eq!(
        count(&panel.state.pool, "runtime_deployment_bindings").await?,
        1
    );
    sqlx::query(
        "UPDATE deployments SET bundle_sha256=$3 WHERE server_id=$1 AND module=$2 AND rev=1",
    )
    .bind(server)
    .bind(MODULE)
    .bind("c".repeat(64))
    .execute(&panel.state.pool)
    .await?;
    assert!(
        runtime_control::request_checkpoint(&panel.state, server, MODULE)
            .await
            .is_err()
    );
    runtime_control::record_checkpoint_result(&panel.state, server, success(&first)).await?;
    assert_eq!(
        outcome(&panel.state.pool, first.request_id).await?,
        "superseded"
    );
    assert_eq!(health(&panel.state.pool, server).await?, (0, false));
    Ok(())
}
