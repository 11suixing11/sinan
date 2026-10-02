use super::{
    account_on, billing,
    client::Cloud,
    lock,
    model::{Resource, Target},
    operations,
};
use crate::error::ApiResult;
use sqlx::{PgPool, types::Json};
use std::time::Duration;
use uuid::Uuid;

pub(super) async fn refresh(pool: &PgPool, id: Uuid, cloud: &Cloud) -> ApiResult<()> {
    let mut tx = lock(pool, id).await?;
    let mut account = account_on(&mut tx, id).await?;
    let now = sinan_protocol::now_timestamp();
    if !account.enabled || account.next_run_at > now {
        return Ok(());
    }
    sqlx::query("UPDATE alicloud_accounts SET last_attempt_at=$2,next_run_at=$3 WHERE id=$1")
        .bind(id)
        .bind(now)
        .bind(now + 300)
        .execute(&mut *tx)
        .await?;
    match cloud.bill(&account, now).await {
        Ok(bill) => {
            sqlx::query("UPDATE alicloud_accounts SET bill=$2,error_code=NULL WHERE id=$1")
                .bind(id)
                .bind(Json(&bill))
                .execute(&mut *tx)
                .await?;
            account.bill = Some(Json(bill));
            account.error_code = None;
        }
        Err(error) => {
            sqlx::query("UPDATE alicloud_accounts SET error_code=$2,next_run_at=GREATEST(next_run_at,$3) WHERE id=$1")
                .bind(id).bind(error.code).bind(now+error.retry_after.max(300)).execute(&mut *tx).await?;
            account.error_code = Some(error.code.into());
        }
    }
    // Compatibility counters are display-only: no trustworthy period field is supplied.
    match cloud.traffic(&account, now).await {
        Ok(traffic) => {
            sqlx::query("UPDATE alicloud_accounts SET traffic=$2,traffic_error=NULL WHERE id=$1")
                .bind(id)
                .bind(Json(traffic))
                .execute(&mut *tx)
                .await?;
        }
        Err(error) => {
            sqlx::query("UPDATE alicloud_accounts SET traffic_error=$2 WHERE id=$1")
                .bind(id)
                .bind(error.code)
                .execute(&mut *tx)
                .await?;
        }
    }
    let resources: Vec<Resource> = sqlx::query_as("SELECT * FROM alicloud_resources WHERE account_id=$1 AND NOT archived ORDER BY last_attempt_at,id LIMIT 8")
        .bind(id).fetch_all(&mut *tx).await?;
    for resource in resources {
        // Poll at most eight resources per pass; rotate by last attempt, including failures.
        sqlx::query("UPDATE alicloud_resources SET last_attempt_at=$2 WHERE id=$1")
            .bind(resource.id)
            .bind(sinan_protocol::now_timestamp())
            .execute(&mut *tx)
            .await?;
        let snapshot = match cloud.snapshot(&account, &resource).await {
            Ok(snapshot) => snapshot,
            Err(error) => {
                sqlx::query("UPDATE alicloud_resources SET error_code=$2 WHERE id=$1")
                    .bind(resource.id)
                    .bind(error.code)
                    .execute(&mut *tx)
                    .await?;
                continue;
            }
        };
        operations::snapshot_on(&mut tx, resource.id, &snapshot).await?;
        if !resource.auto_enabled
            || !billing::exceeded(&account, sinan_protocol::now_timestamp())
            || snapshot.charge_type != "PayByTraffic"
            || snapshot.bandwidth_mbps <= resource.cap_mbps
        {
            continue;
        }
        let cycle = billing::month(now);
        let existing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM alicloud_operations WHERE resource_id=$1 AND (status IN ('queued','running','uncertain') OR (source='automatic' AND billing_cycle=$2 AND account_revision=$3 AND resource_revision=$4)))")
            .bind(resource.id).bind(&cycle).bind(account.revision).bind(resource.revision).fetch_one(&mut *tx).await?;
        if existing {
            continue;
        }
        let target = Target {
            bandwidth_mbps: resource.cap_mbps,
            charge_type: "PayByTraffic".into(),
        };
        operations::prepare(
            &mut tx,
            &account,
            &resource,
            &snapshot,
            &target,
            Some(&cycle),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn run(pool: PgPool) {
    let mut timer = tokio::time::interval(Duration::from_secs(5));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        if let Err(error) = tick(&pool).await {
            tracing::warn!(%error,"Cloud management maintenance failed");
        }
    }
}

async fn tick(pool: &PgPool) -> ApiResult<()> {
    let cloud = Cloud::new().map_err(super::failure)?;
    let now = sinan_protocol::now_timestamp();
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM alicloud_operations WHERE status IN ('queued','running','uncertain') AND next_check_at<=$1 ORDER BY next_check_at,created_at LIMIT 8")
        .bind(now).fetch_all(pool).await?;
    for id in ids {
        if let Err(error) = operations::process(pool, id, &cloud).await {
            tracing::warn!(%error,%id,"Cloud operation reconciliation failed");
        }
    }
    let accounts: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM alicloud_accounts WHERE enabled AND NOT archived AND next_run_at<=$1 ORDER BY next_run_at,id LIMIT 1")
        .bind(now).fetch_all(pool).await?;
    for id in accounts {
        refresh(pool, id, &cloud).await?;
    }
    sqlx::query("UPDATE alicloud_operations SET status='cancelled',updated_at=$1 WHERE status='preview' AND expires_at<=$1").bind(now).execute(pool).await?;
    sqlx::query("DELETE FROM alicloud_operations WHERE status IN ('preview','cancelled','dismissed','failed','succeeded') AND created_at<$1")
        .bind(now-180*86400).execute(pool).await?;
    Ok(())
}
