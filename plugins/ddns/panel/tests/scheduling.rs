use super::{provider::Mock, *};
use crate::plugins::ddns::{editable, load, worker};
use sqlx::PgPool;

async fn seed(pool: &PgPool) -> anyhow::Result<Uuid> {
    let now = sinan_protocol::now_timestamp();
    let server: i64 = sqlx::query_scalar("INSERT INTO servers(name,static_info,last_seen,static_info_received_at) VALUES('TEST_ONLY DDNS',$1,$2,$2) RETURNING id")
        .bind(json!({"ip_addresses":[public_ip(4)]})).bind(now).fetch_one(pool).await?;
    let mut rule = rule();
    rule.config.server_id = server;
    super::super::settings::set_enabled(pool, server, true).await?;
    sqlx::query("INSERT INTO ddns_rules(id,server_id,config,api_token) VALUES($1,$2,$3,$4)")
        .bind(rule.id)
        .bind(server)
        .bind(json!(rule.config))
        .bind(TOKEN)
        .execute(pool)
        .await?;
    Ok(rule.id)
}

async fn due(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query("UPDATE ddns_rules SET attempted_at=NULL,next_run_at=0 WHERE id=$1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

#[sqlx::test]
async fn plugin_disable_blocks_claims_preserves_rules_and_cannot_acknowledge_during_io(
    pool: PgPool,
) -> anyhow::Result<()> {
    let id = seed(&pool).await?;
    let rule = load(&pool, id).await?;
    let mock = Mock::start().await;
    sqlx::query("UPDATE ddns_rules SET lease_until=$1")
        .bind(sinan_protocol::now_timestamp() + 60)
        .execute(&pool)
        .await?;
    assert!(
        super::super::settings::set_enabled(&pool, rule.config.server_id, false)
            .await
            .is_err()
    );
    sqlx::query("UPDATE ddns_rules SET lease_until=0")
        .execute(&pool)
        .await?;
    super::super::settings::set_enabled(&pool, rule.config.server_id, false).await?;
    worker::sync_with(&pool, id, false, &mock.client).await?;
    assert!(
        worker::sync_with(&pool, id, true, &mock.client)
            .await
            .is_err()
    );
    assert!(mock.data.lock().unwrap().requests.is_empty());
    assert!(load(&pool, id).await?.config.enabled);
    let view = model::view(&pool, load(&pool, id).await?).await?;
    assert_eq!(view["plugin_enabled"], false);
    assert_eq!(view["ip_status"], "plugin_disabled");
    super::super::settings::set_enabled(&pool, rule.config.server_id, true).await?;
    worker::sync_with(&pool, id, false, &mock.client).await?;
    assert_eq!(mock.writes(), 1);
    Ok(())
}

#[sqlx::test]
async fn scheduler_persists_success_and_does_not_repeat_unchanged_writes(
    pool: PgPool,
) -> anyhow::Result<()> {
    let id = seed(&pool).await?;
    let mock = Mock::start().await;
    worker::sync_with(&pool, id, false, &mock.client).await?;
    let row = load(&pool, id).await?;
    assert_eq!(row.status, "updated");
    assert_eq!(
        row.last_ip.as_deref(),
        Some(public_ip(4).to_string().as_str())
    );
    assert!(row.last_success_at.is_some());
    assert_eq!(mock.writes(), 1);
    assert!(
        worker::sync_with(&pool, id, true, &mock.client)
            .await
            .is_err()
    );
    worker::sync_with(&pool, id, false, &mock.client).await?;
    assert_eq!(mock.data.lock().unwrap().requests.len(), 3);
    due(&pool, id).await?;
    worker::sync_with(&pool, id, false, &mock.client).await?;
    assert_eq!(load(&pool, id).await?.status, "unchanged");
    assert_eq!(mock.writes(), 1);
    Ok(())
}

#[sqlx::test]
async fn missing_stale_offline_and_retired_ips_preserve_dns_without_external_requests(
    pool: PgPool,
) -> anyhow::Result<()> {
    let id = seed(&pool).await?;
    let mock = Mock::start().await;
    worker::sync_with(&pool, id, false, &mock.client).await?;
    let original = load(&pool, id).await?;
    mock.data.lock().unwrap().requests.clear();
    for (statement, code) in [
        ("UPDATE servers SET static_info='{}'", "no_public_ip"),
        (
            "UPDATE servers SET static_info_received_at=NULL",
            "ip_stale",
        ),
        ("UPDATE servers SET last_seen=NULL", "server_offline"),
        ("UPDATE servers SET deleted_at=1", "server_retired"),
    ] {
        sqlx::query(statement).execute(&pool).await?;
        due(&pool, id).await?;
        worker::sync_with(&pool, id, false, &mock.client).await?;
        let current = load(&pool, id).await?;
        assert_eq!(current.error_code.as_deref(), Some(code));
        assert_eq!(current.status, "waiting");
        assert_eq!(current.last_ip, original.last_ip);
        assert_eq!(current.last_success_at, original.last_success_at);
    }
    assert!(mock.data.lock().unwrap().requests.is_empty());
    assert_eq!(mock.data.lock().unwrap().records.len(), 1);
    Ok(())
}

#[sqlx::test]
async fn leases_exclude_parallel_work_block_edits_and_recover_after_expiry(
    pool: PgPool,
) -> anyhow::Result<()> {
    let id = seed(&pool).await?;
    let mock = Mock::start().await;
    sqlx::query("UPDATE ddns_rules SET lease_id=$2,lease_until=$3,status='running' WHERE id=$1")
        .bind(id)
        .bind(Uuid::new_v4())
        .bind(sinan_protocol::now_timestamp() + 60)
        .execute(&pool)
        .await?;
    let mut tx = pool.begin().await?;
    assert!(editable(&mut tx, id).await.is_err());
    tx.rollback().await?;
    assert!(
        worker::sync_with(&pool, id, true, &mock.client)
            .await
            .is_err()
    );
    assert!(mock.data.lock().unwrap().requests.is_empty());
    sqlx::query("UPDATE ddns_rules SET lease_until=0 WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await?;
    let (first, second) = tokio::join!(
        worker::sync_with(&pool, id, true, &mock.client),
        worker::sync_with(&pool, id, true, &mock.client)
    );
    assert_ne!(first.is_ok(), second.is_ok());
    assert_eq!(mock.writes(), 1);
    assert_eq!(mock.data.lock().unwrap().requests.len(), 3);
    assert_eq!(load(&pool, id).await?.lease_until, 0);
    Ok(())
}

#[sqlx::test]
async fn rate_limit_backoff_survives_retries_without_exposing_response_or_losing_success(
    pool: PgPool,
) -> anyhow::Result<()> {
    let id = seed(&pool).await?;
    let mock = Mock::start().await;
    worker::sync_with(&pool, id, false, &mock.client).await?;
    let original = load(&pool, id).await?;
    due(&pool, id).await?;
    mock.data.lock().unwrap().reply = Some((429, TOKEN.into()));
    worker::sync_with(&pool, id, false, &mock.client).await?;
    let current = load(&pool, id).await?;
    assert_eq!(current.error_code.as_deref(), Some("rate_limited"));
    assert_eq!(current.last_success_at, original.last_success_at);
    assert!(current.next_run_at >= sinan_protocol::now_timestamp() + 899);
    sqlx::query("UPDATE ddns_rules SET attempted_at=NULL")
        .execute(&pool)
        .await?;
    assert!(
        worker::sync_with(&pool, id, true, &mock.client)
            .await
            .is_err()
    );
    worker::sync_with(&pool, id, false, &mock.client).await?;
    assert_eq!(mock.data.lock().unwrap().requests.len(), 4);
    assert!(
        !model::view(&pool, current)
            .await?
            .to_string()
            .contains(TOKEN)
    );
    Ok(())
}
