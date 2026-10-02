#![forbid(unsafe_code)]
mod business_support;
mod probe_support;
#[path = "../../protocol/tests/support/release.rs"]
mod release_support;

use anyhow::Result;
use business_support::TestPanel;
use reqwest::{Method, StatusCode};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sinan_panel::probes::ConfiguredProbe;
use sinan_protocol::{
    PROBE_LEASE_CAPABILITY, ProbeBatch, ProbeExecution, ProbeLease, ProbeResult, TaskAck,
    now_timestamp, telemetry::now_millis,
};
use sqlx::PgPool;
use uuid::Uuid;

fn spec() -> Value {
    json!({"id":Uuid::nil(),"name":"授权夹具","kind":"tcp","target":"probe.example.test",
        "port":443,"interval_secs":10,"carrier":"fixture","enabled":true})
}

async fn capable(panel: &TestPanel, server: i64) -> Result<()> {
    sqlx::query("UPDATE servers SET capabilities=capabilities || $2 WHERE id=$1")
        .bind(server)
        .bind(json!([PROBE_LEASE_CAPABILITY]))
        .execute(&panel.state.pool)
        .await?;
    Ok(())
}

async fn lease(panel: &TestPanel, token: &str) -> Result<ProbeLease> {
    Ok(panel
        .client
        .get(format!("{}/api/agent/v1/probe-lease", panel.base))
        .bearer_auth(token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?)
}

async fn configured(panel: &TestPanel, cookie: &str, server: i64) -> Result<ConfiguredProbe> {
    Ok(panel
        .admin(
            Method::POST,
            &format!("/api/servers/{server}/probes"),
            cookie,
            Some(probe_support::configured(spec())),
        )
        .await?
        .error_for_status()?
        .json()
        .await?)
}

fn sample(lease: &ProbeLease) -> ProbeResult {
    ProbeResult {
        id: Uuid::new_v4(),
        probe_id: lease.probes[0].spec.id,
        sampled_at: now_millis(),
        latency_ms: Some(1.0),
        loss_percent: 0.0,
        error: None,
        execution: Some(ProbeExecution {
            lease_id: lease.id,
            revision: lease.revision,
            issued_at: lease.issued_at,
            expires_at: lease.expires_at,
            probe: lease.probes[0].clone(),
        }),
    }
}

async fn ingest(
    panel: &TestPanel,
    token: &str,
    results: Vec<ProbeResult>,
) -> Result<reqwest::Response> {
    Ok(panel
        .client
        .post(format!("{}/api/agent/v1/probe-results", panel.base))
        .bearer_auth(token)
        .json(&ProbeBatch { results })
        .send()
        .await?)
}

#[sqlx::test]
async fn unknown_authority_is_history_only_and_admin_edits_use_revision_cas(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel
        .authenticated_device(&cookie, "authorization gate")
        .await?;
    let path = format!("/api/servers/{server}/probes");
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/agent/v1/probe-lease", panel.base))
            .send()
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        panel
            .admin(Method::POST, &path, &cookie, Some(spec()))
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    for changes in [
        json!({"source":""}),
        json!({"evidence":""}),
        json!({"expires_at":now_timestamp()-1}),
    ] {
        let mut input = probe_support::configured(spec());
        for (key, value) in changes.as_object().unwrap() {
            input["authorization"][key] = value.clone();
        }
        assert_eq!(
            panel
                .admin(Method::POST, &path, &cookie, Some(input))
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        panel
            .admin(
                Method::POST,
                "/api/servers",
                &cookie,
                Some(json!({"name":"unauthorized initial","probes":[spec()]}))
            )
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        panel
            .admin(
                Method::POST,
                "/api/latency-tasks",
                &cookie,
                Some(json!({"spec":spec(),"server_ids":[server],"default_enabled":false}))
            )
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut disabled = spec();
    disabled["enabled"] = json!(false);
    let legacy: ConfiguredProbe = panel
        .admin(Method::POST, &path, &cookie, Some(disabled))
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(legacy.authorization.is_none());
    assert_eq!(legacy.revision, Some(1));
    let old_wire: Vec<Value> = panel
        .client
        .get(format!("{}/api/agent/v1/probes", panel.base))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(old_wire.is_empty());
    assert_eq!(
        panel
            .client
            .get(format!("{}/api/agent/v1/probe-lease", panel.base))
            .bearer_auth(&ack.session_token)
            .send()
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    capable(&panel, server).await?;
    assert!(lease(&panel, &ack.session_token).await?.probes.is_empty());
    let edit_path = format!("{path}/{}", legacy.spec.id);
    let mut enabled = serde_json::to_value(&legacy)?;
    enabled["enabled"] = json!(true);
    assert_eq!(
        panel
            .admin(Method::PATCH, &edit_path, &cookie, Some(enabled.clone()))
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    enabled["authorization"] = probe_support::authorization();
    let saved: ConfiguredProbe = panel
        .admin(Method::PATCH, &edit_path, &cookie, Some(enabled.clone()))
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(saved.revision, Some(2));
    assert_eq!(
        panel
            .admin(Method::PATCH, &edit_path, &cookie, Some(enabled))
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        panel
            .admin(
                Method::DELETE,
                &edit_path,
                &cookie,
                Some(json!({"revision":1}))
            )
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        panel
            .admin(Method::DELETE, &edit_path, &cookie, None)
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(lease(&panel, &ack.session_token).await?.probes.len(), 1);
    Ok(())
}

#[sqlx::test]
async fn issued_lease_is_scoped_reused_bounded_and_legacy_wire_stays_eight_fields(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel
        .authenticated_device(&cookie, "lease issuance")
        .await?;
    let (other, _other_socket, other_ack) =
        panel.authenticated_device(&cookie, "other lease").await?;
    for id in [server, other] {
        capable(&panel, id).await?;
    }
    let mut input = probe_support::configured(spec());
    input["authorization"]["scope"] = json!("third_party");
    input["authorization"]["evidence"] = json!("TEST_ONLY private permission note");
    input["authorization"]["expires_at"] = json!(now_timestamp() + 40);
    panel
        .admin(
            Method::POST,
            &format!("/api/servers/{server}/probes"),
            &cookie,
            Some(input),
        )
        .await?
        .error_for_status()?;
    let first = lease(&panel, &ack.session_token).await?;
    assert!(first.valid());
    assert_eq!(first.server_id, server);
    assert_eq!(first.probes.len(), 1);
    assert!(first.expires_at <= first.probes[0].authorization.expires_at.unwrap());
    let repeated = lease(&panel, &ack.session_token).await?;
    assert_eq!(first, repeated);
    assert!(
        lease(&panel, &other_ack.session_token)
            .await?
            .probes
            .is_empty()
    );
    let wire: Vec<Value> = panel
        .client
        .get(format!("{}/api/agent/v1/probes", panel.base))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(wire[0].as_object().unwrap().len(), 8);
    assert!(wire[0].get("authorization").is_none());
    assert!(wire[0].get("revision").is_none());
    let receipts: Vec<Value> =
        sqlx::query_scalar("SELECT probe_digests FROM probe_leases WHERE server_id=$1")
            .bind(server)
            .fetch_all(&panel.state.pool)
            .await?;
    assert_eq!(receipts.len(), 1);
    assert!(!serde_json::to_string(&receipts)?.contains("private permission note"));
    // Old fixture receipts stay isolated and are pruned only for this owner.
    for owner in [server, other] {
        sqlx::query("INSERT INTO probe_leases(id,server_id,revision,issued_at,expires_at,probe_digests) SELECT md5(($1::bigint)::text || '-' || i::text)::uuid,$1,0,$2::bigint-i,$2::bigint-i+90,'{}'::jsonb FROM generate_series(1,513) i")
            .bind(owner).bind(now_timestamp()-12_000).execute(&panel.state.pool).await?;
    }
    sqlx::query(
        "UPDATE probe_leases SET issued_at=issued_at-30,expires_at=expires_at-30 WHERE id=$1",
    )
    .bind(first.id)
    .execute(&panel.state.pool)
    .await?;
    lease(&panel, &ack.session_token).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM probe_leases WHERE server_id=$1")
        .bind(server)
        .fetch_one(&panel.state.pool)
        .await?;
    assert!(count <= 512);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM probe_leases WHERE server_id=$1")
            .bind(other)
            .fetch_one(&panel.state.pool)
            .await?,
        514
    );
    Ok(())
}

#[sqlx::test]
async fn exact_issuance_controls_new_results_and_saved_duplicates_survive_revocation(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel.authenticated_device(&cookie, "lease results").await?;
    let (_other, _other_socket, other_ack) = panel
        .authenticated_device(&cookie, "foreign results")
        .await?;
    capable(&panel, server).await?;
    let probe = configured(&panel, &cookie, server).await?;
    let issued = lease(&panel, &ack.session_token).await?;
    let accepted = sample(&issued);
    let acked: TaskAck = ingest(&panel, &ack.session_token, vec![accepted.clone()])
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(acked.ids, vec![accepted.id]);
    let before: (Value, String) =
        sqlx::query_as("SELECT result,digest FROM probe_results WHERE id=$1")
            .bind(accepted.id)
            .fetch_one(&panel.state.pool)
            .await?;
    let mut wrong = sample(&issued);
    wrong.execution.as_mut().unwrap().probe.spec.target = "other.example.test".into();
    let foreign = sample(&issued);
    let mut missing = sample(&issued);
    missing.execution.as_mut().unwrap().lease_id = Uuid::new_v4();
    let mut expired = sample(&issued);
    let execution = expired.execution.as_mut().unwrap();
    execution.lease_id = Uuid::new_v4();
    execution.issued_at = now_timestamp() - 3 * 3_600 - 1;
    execution.expires_at = execution.issued_at + 90;
    expired.sampled_at = execution.issued_at * 1_000 + 1;
    sqlx::query("INSERT INTO probe_leases(id,server_id,revision,issued_at,expires_at,probe_digests) SELECT $1,server_id,revision,$2,$3,probe_digests FROM probe_leases WHERE id=$4")
        .bind(execution.lease_id).bind(execution.issued_at).bind(execution.expires_at).bind(issued.id)
        .execute(&panel.state.pool).await?;
    for (token, result) in [
        (&ack.session_token, wrong),
        (&other_ack.session_token, foreign),
        (&ack.session_token, missing),
        (&ack.session_token, expired),
    ] {
        let acked: TaskAck = ingest(&panel, token, vec![result.clone()])
            .await?
            .error_for_status()?
            .json()
            .await?;
        assert_eq!(acked.ids, vec![result.id]);
        assert!(
            !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM probe_results WHERE id=$1)"
            )
            .bind(result.id)
            .fetch_one(&panel.state.pool)
            .await?
        );
    }
    let mut malformed = sample(&issued);
    malformed.sampled_at = issued.expires_at * 1_000;
    assert_eq!(
        ingest(&panel, &ack.session_token, vec![malformed])
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut wrong_id = sample(&issued);
    wrong_id.execution.as_mut().unwrap().probe.spec.id = Uuid::new_v4();
    assert_eq!(
        ingest(&panel, &ack.session_token, vec![wrong_id])
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    let mut revoked = serde_json::to_value(probe)?;
    revoked["enabled"] = json!(false);
    revoked["authorization"] = Value::Null;
    panel
        .admin(
            Method::PATCH,
            &format!("/api/servers/{server}/probes/{}", accepted.probe_id),
            &cookie,
            Some(revoked),
        )
        .await?
        .error_for_status()?;
    let late = sample(&issued);
    let acked: TaskAck = ingest(
        &panel,
        &ack.session_token,
        vec![late.clone(), accepted.clone()],
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    assert_eq!(acked.ids, vec![late.id, accepted.id]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM probe_results")
            .fetch_one(&panel.state.pool)
            .await?,
        1
    );
    assert_eq!(
        sqlx::query_as::<_, (Value, String)>("SELECT result,digest FROM probe_results WHERE id=$1")
            .bind(accepted.id)
            .fetch_one(&panel.state.pool)
            .await?,
        before
    );
    let mut changed = accepted;
    changed.loss_percent = 100.0;
    assert_eq!(
        ingest(&panel, &ack.session_token, vec![changed])
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    Ok(())
}

#[sqlx::test]
async fn expiry_changes_revision_and_anonymous_projection_removes_authorization_evidence(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel
        .authenticated_device(&cookie, "public permission")
        .await?;
    capable(&panel, server).await?;
    let probe = configured(&panel, &cookie, server).await?;
    let first = lease(&panel, &ack.session_token).await?;
    let accepted = sample(&first);
    ingest(&panel, &ack.session_token, vec![accepted])
        .await?
        .error_for_status()?;
    panel.admin(Method::PATCH, "/api/settings", &cookie,
        Some(json!({"public_dashboard":true,"offline_alerts":false,"offline_minutes":2,"telegram_enabled":false,"telegram_chat_id":""})))
        .await?.error_for_status()?;
    for endpoint in [
        "/api/dashboard/probes/overview".into(),
        format!("/api/dashboard/servers/{server}/probe-results"),
    ] {
        let text = panel
            .client
            .get(format!("{}{endpoint}", panel.base))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        for private in [
            "evidence",
            "execution",
            "owned fixture",
            "probe.example.test",
        ] {
            assert!(!text.contains(private));
        }
    }
    let allowed: Value = panel
        .client
        .get(format!("{}/api/dashboard/probes/overview", panel.base))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(allowed[0]["authorization_state"], "allowed");
    sqlx::query("UPDATE network_probes SET target_authorization=jsonb_set(target_authorization,'{expires_at}',to_jsonb($2::bigint)) WHERE id=$1")
        .bind(probe.spec.id).bind(now_timestamp()-1).execute(&panel.state.pool).await?;
    let expired = lease(&panel, &ack.session_token).await?;
    assert!(expired.probes.is_empty());
    assert!(expired.revision > first.revision);
    let state: Value = panel
        .client
        .get(format!("{}/api/dashboard/probes/overview", panel.base))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(state[0]["authorization_state"], "expired");
    sqlx::query("UPDATE network_probes SET target_authorization=NULL WHERE id=$1")
        .bind(probe.spec.id)
        .execute(&panel.state.pool)
        .await?;
    let state: Value = panel
        .client
        .get(format!("{}/api/dashboard/probes/overview", panel.base))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert_eq!(state[0]["authorization_state"], "missing");
    Ok(())
}

#[sqlx::test]
async fn authorization_migration_preserves_0025_cleanup_and_legacy_probe_history(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool.clone()).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel
        .authenticated_device(&cookie, "migration history")
        .await?;
    let probe = Uuid::new_v4();
    let mut old_spec = spec();
    old_spec["id"] = json!(probe);
    sqlx::query("INSERT INTO network_probes(id,server_id,spec) VALUES($1,$2,$3)")
        .bind(probe)
        .bind(server)
        .bind(&old_spec)
        .execute(&pool)
        .await?;
    let old = json!({"id":Uuid::new_v4(),"probe_id":probe,"sampled_at":now_millis(),
        "latency_ms":null,"loss_percent":100.0,"error":"TEST_ONLY old unknown"});
    let result_id = Uuid::parse_str(old["id"].as_str().unwrap())?;
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&serde_json::from_value::<ProbeResult>(
            old.clone()
        )?)?)
    );
    sqlx::query("INSERT INTO probe_results(id,server_id,probe_id,sampled_at,result,digest) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(result_id).bind(server).bind(probe).bind(old["sampled_at"].as_i64().unwrap())
        .bind(&old).bind(&digest).execute(&pool).await?;
    let job = Uuid::new_v4();
    sqlx::query("INSERT INTO diagnostic_jobs(id,server_id,job,status,report,created_at,updated_at,expires_at) VALUES($1,$2,'{}','cleaning',$3,1,1,1)")
        .bind(job).bind(server).bind(json!({"text":"TEST_ONLY preserved cleanup report"})).execute(&pool).await?;
    sqlx::raw_sql("DROP TABLE probe_leases; ALTER TABLE network_probes DROP COLUMN target_authorization,DROP COLUMN revision; ALTER TABLE latency_tasks DROP COLUMN target_authorization; ALTER TABLE servers DROP COLUMN probe_revision,DROP COLUMN probe_fingerprint; DELETE FROM _sqlx_migrations WHERE version=26;")
        .execute(&pool).await?;
    sqlx::migrate!("./migrations").run(&pool).await?;
    let unchanged: (Value, Option<Value>, i64) =
        sqlx::query_as("SELECT spec,target_authorization,revision FROM network_probes WHERE id=$1")
            .bind(probe)
            .fetch_one(&pool)
            .await?;
    assert_eq!(unchanged, (old_spec, None, 1));
    let preserved: (Value, String) =
        sqlx::query_as("SELECT result,digest FROM probe_results WHERE id=$1")
            .bind(result_id)
            .fetch_one(&pool)
            .await?;
    assert_eq!(preserved, (old.clone(), digest));
    let cleanup: (String, bool, Value) =
        sqlx::query_as("SELECT status,agent_completed,report FROM diagnostic_jobs WHERE id=$1")
            .bind(job)
            .fetch_one(&pool)
            .await?;
    assert_eq!(
        cleanup,
        (
            "cleaning".into(),
            false,
            json!({"text":"TEST_ONLY preserved cleanup report"})
        )
    );
    let applied: Vec<i64> = sqlx::query_scalar("SELECT version FROM _sqlx_migrations WHERE version IN (25,26) AND success ORDER BY version")
        .fetch_all(&pool).await?;
    assert_eq!(applied, vec![25, 26]);
    let wire: Vec<Value> = panel
        .client
        .get(format!("{}/api/agent/v1/probes", panel.base))
        .bearer_auth(&ack.session_token)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    assert!(wire.is_empty());
    let duplicate: ProbeResult = serde_json::from_value(old)?;
    ingest(&panel, &ack.session_token, vec![duplicate])
        .await?
        .error_for_status()?;
    Ok(())
}

#[sqlx::test]
async fn upgraded_devices_drain_legacy_unproved_outbox_without_accepting_new_history(
    pool: PgPool,
) -> Result<()> {
    let panel = TestPanel::start(pool).await?;
    let cookie = panel.admin_cookie().await?;
    let (server, _socket, ack) = panel
        .authenticated_device(&cookie, "upgrade outbox")
        .await?;
    configured(&panel, &cookie, server).await?;
    capable(&panel, server).await?;
    let issued = lease(&panel, &ack.session_token).await?;
    let mut saved = sample(&issued);
    saved.execution = None;
    let saved_value = serde_json::to_value(&saved)?;
    let saved_digest = format!("{:x}", Sha256::digest(serde_json::to_vec(&saved)?));
    sqlx::query("INSERT INTO probe_results(id,server_id,probe_id,sampled_at,result,digest) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(saved.id).bind(server).bind(saved.probe_id).bind(saved.sampled_at)
        .bind(&saved_value).bind(&saved_digest).execute(&panel.state.pool).await?;
    let mut unproved = sample(&issued);
    unproved.execution = None;
    let fresh = sample(&issued);
    let acked: TaskAck = ingest(
        &panel,
        &ack.session_token,
        vec![unproved.clone(), fresh.clone(), saved.clone()],
    )
    .await?
    .error_for_status()?
    .json()
    .await?;
    assert_eq!(acked.ids, vec![unproved.id, fresh.id, saved.id]);
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM probe_results WHERE id=$1)")
            .bind(unproved.id)
            .fetch_one(&panel.state.pool)
            .await?
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM probe_results WHERE server_id=$1")
            .bind(server)
            .fetch_one(&panel.state.pool)
            .await?,
        2
    );
    assert_eq!(
        sqlx::query_as::<_, (Value, String)>("SELECT result,digest FROM probe_results WHERE id=$1")
            .bind(saved.id)
            .fetch_one(&panel.state.pool)
            .await?,
        (saved_value, saved_digest)
    );
    let proved: Value = sqlx::query_scalar("SELECT result FROM probe_results WHERE id=$1")
        .bind(fresh.id)
        .fetch_one(&panel.state.pool)
        .await?;
    assert_eq!(proved, serde_json::to_value(fresh)?);
    Ok(())
}
