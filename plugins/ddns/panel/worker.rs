use super::{
    LEASE_SECS, REQUEST_BUDGET,
    cloudflare::{Cloudflare, Failure, Outcome},
    load,
    model::{self, Rule},
};
use crate::error::{ApiError, ApiResult};
use futures_util::{StreamExt, stream};
use sqlx::PgPool;
use std::{net::IpAddr, time::Duration};
use uuid::Uuid;

async fn claim(pool: &PgPool, id: Uuid, force: bool) -> ApiResult<Option<(Uuid, Rule)>> {
    let now = sinan_protocol::now_timestamp();
    let mut tx = pool.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock(739104824)")
        .execute(&mut *tx)
        .await?;
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM ddns_rules WHERE lease_until>$1")
        .bind(now)
        .fetch_one(&mut *tx)
        .await?;
    if active >= 2 {
        return if force { Err(ApiError::Busy) } else { Ok(None) };
    }
    let Some(rule) =
        sqlx::query_as::<_, Rule>("SELECT * FROM ddns_rules WHERE id=$1 FOR UPDATE SKIP LOCKED")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?
    else {
        return if force {
            load(pool, id).await?;
            Err(ApiError::Busy)
        } else {
            Ok(None)
        };
    };
    if !rule.config.enabled {
        return if force {
            Err(ApiError::Conflict("请先启用规则".into()))
        } else {
            Ok(None)
        };
    }
    let enabled: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM server_plugins WHERE server_id=$1 AND plugin='ddns' AND enabled)")
        .bind(rule.config.server_id).fetch_one(&mut *tx).await?;
    if !enabled {
        return if force {
            Err(ApiError::Conflict("请先为该服务器启用 DDNS 插件".into()))
        } else {
            Ok(None)
        };
    }
    if rule.lease_until > now
        || (force && rule.attempted_at.is_some_and(|at| at + 60 > now))
        || (rule.next_run_at > now && (!force || rule.failures > 0))
    {
        return if force { Err(ApiError::Busy) } else { Ok(None) };
    }
    let lease = Uuid::new_v4();
    sqlx::query("UPDATE ddns_rules SET lease_id=$2,lease_until=$3,attempted_at=$4,next_run_at=$5,status='running' WHERE id=$1")
        .bind(id).bind(lease).bind(now+LEASE_SECS).bind(now).bind(now+i64::from(rule.config.interval_secs)).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Some((lease, rule)))
}

async fn complete(
    pool: &PgPool,
    rule: &Rule,
    lease: Uuid,
    result: Result<(IpAddr, Outcome), Failure>,
) -> ApiResult<()> {
    let now = sinan_protocol::now_timestamp();
    match result {
        Ok((ip, outcome)) => {
            sqlx::query("UPDATE ddns_rules SET record_id=$3,last_ip=$4,last_success_at=$5,status=$6,error_code=NULL,failures=0,lease_id=NULL,lease_until=0,next_run_at=$7 WHERE id=$1 AND lease_id=$2")
                .bind(rule.id).bind(lease).bind(outcome.record_id).bind(ip.to_string()).bind(now).bind(outcome.status).bind(now+i64::from(rule.config.interval_secs)).execute(pool).await?;
        }
        Err(error) => {
            let waiting = matches!(
                error.code,
                "server_retired" | "server_offline" | "ip_stale" | "no_public_ip"
            );
            let failures = if waiting {
                0
            } else {
                rule.failures.saturating_add(1).min(16)
            };
            let delay = if waiting {
                60
            } else {
                (60_i64 << failures.saturating_sub(1).min(6))
                    .min(3600)
                    .max(error.retry_after)
            };
            sqlx::query("UPDATE ddns_rules SET status=$3,error_code=$4,failures=$5,lease_id=NULL,lease_until=0,next_run_at=$6 WHERE id=$1 AND lease_id=$2")
                .bind(rule.id).bind(lease).bind(if waiting { "waiting" } else { "error" }).bind(error.code).bind(failures).bind(now+delay).execute(pool).await?;
        }
    }
    Ok(())
}

pub(super) async fn sync_with(
    pool: &PgPool,
    id: Uuid,
    force: bool,
    provider: &Cloudflare,
) -> ApiResult<()> {
    let Some((lease, rule)) = claim(pool, id, force).await? else {
        return Ok(());
    };
    let result = tokio::time::timeout(Duration::from_secs(REQUEST_BUDGET), async {
        let info = model::observation(pool, rule.config.server_id)
            .await
            .map_err(|_| Failure::from("storage_error"))?;
        let ip = info
            .select(
                &rule.config,
                rule.last_ip.as_deref(),
                sinan_protocol::now_timestamp(),
            )
            .map_err(Failure::from)?;
        provider
            .reconcile(&rule, ip)
            .await
            .map(|outcome| (ip, outcome))
    })
    .await
    .unwrap_or(Err("request_timeout".into()));
    complete(pool, &rule, lease, result).await
}

pub(super) async fn sync(pool: &PgPool, id: Uuid, force: bool) -> ApiResult<()> {
    let provider = Cloudflare::new()
        .map_err(|_| ApiError::Internal(anyhow::anyhow!("DDNS client initialization failed")))?;
    sync_with(pool, id, force, &provider).await
}

pub async fn run(pool: PgPool) {
    let mut timer = tokio::time::interval(Duration::from_secs(15));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        timer.tick().await;
        if let Err(error) = tick(&pool).await {
            tracing::warn!(%error, "DDNS maintenance failed");
        }
    }
}

async fn tick(pool: &PgPool) -> ApiResult<()> {
    let now = sinan_protocol::now_timestamp();
    let ids: Vec<Uuid> = sqlx::query_scalar("SELECT id FROM ddns_rules d WHERE config->>'enabled'='true' AND next_run_at<=$1 AND lease_until<=$1 AND EXISTS(SELECT 1 FROM server_plugins p WHERE p.server_id=d.server_id AND p.plugin='ddns' AND p.enabled) ORDER BY next_run_at,id LIMIT 8")
        .bind(now).fetch_all(pool).await?;
    let results = stream::iter(
        ids.into_iter()
            .map(|id| async move { sync(pool, id, false).await }),
    )
    .buffer_unordered(2)
    .collect::<Vec<_>>()
    .await;
    for result in results {
        result?;
    }
    Ok(())
}
