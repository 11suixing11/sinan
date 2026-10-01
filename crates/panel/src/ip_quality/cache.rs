use super::{CACHE_SECS, IpQuality, QualityDatabase, QueryFailure, reported_ips};
use crate::error::{ApiError, ApiResult};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sinan_protocol::now_timestamp;
use sqlx::{PgPool, Row};
use std::collections::BTreeMap;

#[cfg(test)]
mod tests;

fn decode<T: DeserializeOwned>(value: Value) -> ApiResult<T> {
    serde_json::from_value(value)
        .map_err(anyhow::Error::from)
        .map_err(ApiError::from)
}

fn encode<T: serde::Serialize>(value: &T) -> ApiResult<Value> {
    serde_json::to_value(value)
        .map_err(anyhow::Error::from)
        .map_err(ApiError::from)
}

fn failure(entry: &QualityDatabase) -> Option<QueryFailure> {
    entry.error.as_ref().map(|message| QueryFailure {
        kind: entry.error_kind,
        message: message.clone(),
        http_status: entry.http_status,
        attempted_at: entry.attempted_at,
        elapsed_ms: entry.elapsed_ms,
    })
}

pub(super) async fn begin_refresh(pool: &PgPool, id: i64, now: i64) -> ApiResult<Vec<String>> {
    let mut tx = pool.begin().await?;
    // Both admission checks run after taking the same durable server lock.
    let row = sqlx::query("SELECT static_info,quality_refresh_started FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
        .bind(id).fetch_optional(&mut *tx).await?.ok_or(ApiError::NotFound)?;
    let ips = reported_ips(&row.get::<Value, _>("static_info"));
    if ips.is_empty() {
        return Err(ApiError::Conflict(
            "Agent 尚未上报 IP 地址，请先升级或等待设备上报".into(),
        ));
    }
    if row
        .get::<Option<i64>, _>("quality_refresh_started")
        .is_some_and(|started| started >= now.saturating_sub(60))
    {
        return Err(ApiError::Conflict(
            "IP 质量查询正在进行，请稍后刷新查看结果".into(),
        ));
    }
    let recent: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM server_ip_quality WHERE server_id=$1 AND last_attempt_at>$2)",
    )
    .bind(id)
    .bind(now.saturating_sub(60))
    .fetch_one(&mut *tx)
    .await?;
    if recent {
        return Err(ApiError::Conflict(
            "刚刚查询过 IP 质量，请至少等待一分钟再刷新".into(),
        ));
    }
    sqlx::query("UPDATE servers SET quality_refresh_started=$2 WHERE id=$1")
        .bind(id)
        .bind(now)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(ips)
}

pub(super) async fn persist(pool: &PgPool, id: i64, quality: &[IpQuality]) -> ApiResult<()> {
    let mut tx = pool.begin().await?;
    // Serialize result writes with admission and deletion, including stale replies.
    let exists =
        sqlx::query("SELECT id FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;
    if exists.is_none() {
        return Err(ApiError::NotFound);
    }
    for entry in quality {
        let attempted_at = entry
            .databases
            .iter()
            .filter_map(|dataset| dataset.attempted_at)
            .max();
        let updated = sqlx::query("INSERT INTO server_ip_quality(server_id,ip,provider,payload,checked_at,last_attempt_at) VALUES($1,$2,$3,$4,$5,$6) ON CONFLICT(server_id,ip,provider) DO UPDATE SET payload=EXCLUDED.payload,checked_at=EXCLUDED.checked_at,last_attempt_at=COALESCE(EXCLUDED.last_attempt_at,server_ip_quality.last_attempt_at) WHERE server_ip_quality.checked_at<=EXCLUDED.checked_at")
            .bind(id).bind(&entry.ip).bind(&entry.provider).bind(encode(entry)?)
            .bind(entry.checked_at).bind(attempted_at).execute(&mut *tx).await?;
        if updated.rows_affected() == 0 {
            continue;
        }
        for dataset in &entry.databases {
            let succeeded = dataset.status == "succeeded" && !dataset.fields.is_empty();
            let success_at = succeeded.then(|| {
                dataset
                    .last_success_at
                    .or(dataset.attempted_at)
                    .unwrap_or(entry.checked_at)
            });
            let fresh_until = success_at.map(|at| at.saturating_add(CACHE_SECS));
            let last_error = failure(dataset).map(|error| encode(&error)).transpose()?;
            let success = succeeded.then(|| encode(dataset)).transpose()?;
            sqlx::query("INSERT INTO server_ip_quality_datasets(server_id,ip,provider,database,checked_at,last_attempt,last_attempt_at,last_success_at,fresh_until,last_error,success_payload) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11) ON CONFLICT(server_id,ip,provider,database) DO UPDATE SET checked_at=EXCLUDED.checked_at,last_attempt=EXCLUDED.last_attempt,last_attempt_at=COALESCE(EXCLUDED.last_attempt_at,server_ip_quality_datasets.last_attempt_at),last_success_at=COALESCE(EXCLUDED.last_success_at,server_ip_quality_datasets.last_success_at),fresh_until=COALESCE(EXCLUDED.fresh_until,server_ip_quality_datasets.fresh_until),last_error=EXCLUDED.last_error,success_payload=COALESCE(EXCLUDED.success_payload,server_ip_quality_datasets.success_payload) WHERE server_ip_quality_datasets.checked_at<=EXCLUDED.checked_at")
                .bind(id).bind(&entry.ip).bind(&entry.provider).bind(&dataset.database)
                .bind(entry.checked_at).bind(encode(dataset)?).bind(dataset.attempted_at)
                .bind(success_at).bind(fresh_until).bind(last_error).bind(success)
                .execute(&mut *tx).await?;
        }
        sqlx::query("UPDATE server_ip_quality AS cache SET last_success_at=summary.last_success_at,fresh_until=summary.fresh_until,last_error=summary.last_error FROM (SELECT MAX(last_success_at) AS last_success_at,CASE WHEN COUNT(success_payload)=COUNT(*) THEN MIN(fresh_until) END AS fresh_until,COALESCE(jsonb_object_agg(database,last_error) FILTER(WHERE last_error IS NOT NULL),'{}'::jsonb) AS last_error FROM server_ip_quality_datasets WHERE server_id=$1 AND ip=$2 AND provider=$3) AS summary WHERE cache.server_id=$1 AND cache.ip=$2 AND cache.provider=$3")
            .bind(id).bind(&entry.ip).bind(&entry.provider).execute(&mut *tx).await?;
    }
    // Old addresses remain durable; reads select only the requested/current IPs.
    tx.commit().await?;
    Ok(())
}

pub(super) async fn read(pool: &PgPool, id: i64, ips: &[String]) -> ApiResult<Vec<IpQuality>> {
    let mut tx = pool.begin().await?;
    // Two related reads must see one committed cache generation during refresh.
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await?;
    let rows = sqlx::query("SELECT ip,provider,payload,last_attempt_at,last_success_at,fresh_until,last_error FROM server_ip_quality WHERE server_id=$1 AND ip=ANY($2) ORDER BY ip,provider")
        .bind(id).bind(ips).fetch_all(&mut *tx).await?;
    let datasets = sqlx::query("SELECT ip,provider,database,last_attempt,last_attempt_at,last_success_at,fresh_until,last_error,success_payload FROM server_ip_quality_datasets WHERE server_id=$1 AND ip=ANY($2)")
        .bind(id).bind(ips).fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let datasets: BTreeMap<_, _> = datasets
        .into_iter()
        .map(|row| {
            let key = (
                row.get::<String, _>("ip"),
                row.get::<String, _>("provider"),
                row.get::<String, _>("database"),
            );
            (key, row)
        })
        .collect();
    let now = now_timestamp();
    rows.into_iter()
        .map(|row| {
            let mut entry: IpQuality = decode(row.get("payload"))?;
            entry.provider = row.get("provider");
            entry.last_attempt_at = row.get("last_attempt_at");
            entry.last_success_at = row.get("last_success_at");
            entry.fresh_until = row.get("fresh_until");
            entry.expires_at = entry.fresh_until.unwrap_or(0);
            entry.last_error = decode(row.get("last_error"))?;
            for dataset in &mut entry.databases {
                let key = (
                    entry.ip.clone(),
                    entry.provider.clone(),
                    dataset.database.clone(),
                );
                if let Some(cached) = datasets.get(&key) {
                    *dataset = decode(cached.get("last_attempt"))?;
                    dataset.provider = entry.provider.clone();
                    dataset.target_ip = Some(entry.ip.clone());
                    dataset.last_attempt_at = cached.get("last_attempt_at");
                    dataset.last_success_at = cached.get("last_success_at");
                    dataset.fresh_until = cached.get("fresh_until");
                    dataset.last_error = cached
                        .get::<Option<Value>, _>("last_error")
                        .map(decode)
                        .transpose()?;
                    let success = cached
                        .get::<Option<Value>, _>("success_payload")
                        .map(decode::<QualityDatabase>)
                        .transpose()?;
                    dataset.fields = super::fields::confirmed_cached_fields(
                        &dataset.database,
                        success.map(|saved| saved.fields).unwrap_or_default(),
                    );
                    dataset.historical = !dataset.fields.is_empty()
                        && (dataset.status != "succeeded"
                            || dataset.fresh_until.is_none_or(|until| until <= now));
                }
            }
            Ok(entry)
        })
        .collect()
}
