use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::HeaderMap,
};
use sha2::{Digest, Sha256};
use sinan_protocol::{ProbeBatch, ProbeResult, ProbeSpec, TaskAck, telemetry::now_millis};
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct Overview {
    server_id: i64,
    probe: ProbeSpec,
    results: Vec<ProbeResult>,
}

pub async fn overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Overview>>> {
    auth::require_admin(&state, &headers).await?;
    // One indexed query for all cards; each target has an independent sample budget.
    let rows: Vec<(i64, serde_json::Value, serde_json::Value)> = sqlx::query_as(
        "SELECT p.server_id, p.spec, COALESCE(samples.results, '[]'::jsonb)
         FROM network_probes p JOIN servers s ON s.id=p.server_id AND s.deleted_at IS NULL
         LEFT JOIN LATERAL (
             SELECT jsonb_agg(recent.result ORDER BY recent.sampled_at DESC, recent.id DESC) AS results
             FROM (SELECT id, sampled_at, result FROM probe_results
                   WHERE server_id=p.server_id AND probe_id=p.id AND sampled_at>$1 AND sampled_at<=$2
                   ORDER BY sampled_at DESC, id DESC LIMIT 20) recent
         ) samples ON TRUE ORDER BY p.server_id, p.id",
    )
    .bind(now_millis() - 86_400_000)
    .bind(now_millis())
    .fetch_all(&state.pool).await?;
    Ok(Json(
        rows.into_iter()
            .map(|(server_id, spec, results)| {
                Ok(Overview {
                    server_id,
                    probe: serde_json::from_value(spec)?,
                    results: serde_json::from_value(results)?,
                })
            })
            .collect::<Result<_, serde_json::Error>>()
            .map_err(anyhow::Error::from)?,
    ))
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server): Path<i64>,
) -> ApiResult<Json<Vec<ProbeSpec>>> {
    auth::require_admin(&state, &headers).await?;
    read(&state, server).await
}

async fn read(state: &AppState, server: i64) -> ApiResult<Json<Vec<ProbeSpec>>> {
    let rows: Vec<serde_json::Value> =
        sqlx::query_scalar("SELECT spec FROM network_probes WHERE server_id=$1 ORDER BY id")
            .bind(server)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(
        rows.into_iter()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()
            .map_err(anyhow::Error::from)?,
    ))
}

pub async fn agent_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ProbeSpec>>> {
    let server = auth::require_agent(&state, &headers).await?;
    read(&state, server).await
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server): Path<i64>,
    Json(mut spec): Json<ProbeSpec>,
) -> ApiResult<Json<ProbeSpec>> {
    auth::require_admin(&state, &headers).await?;
    spec.id = Uuid::new_v4();
    if !spec.valid() {
        return Err(ApiError::BadRequest("拨测配置无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT id FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
        .bind(server)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(ApiError::NotFound)?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM network_probes WHERE server_id=$1")
        .bind(server)
        .fetch_one(&mut *tx)
        .await?;
    if count >= 32 {
        return Err(ApiError::Conflict("每台服务器最多配置 32 个拨测".into()));
    }
    sqlx::query("INSERT INTO network_probes(id,server_id,spec) VALUES($1,$2,$3)")
        .bind(spec.id)
        .bind(server)
        .bind(serde_json::to_value(&spec).map_err(anyhow::Error::from)?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(spec))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server, id)): Path<(i64, Uuid)>,
    Json(spec): Json<ProbeSpec>,
) -> ApiResult<Json<ProbeSpec>> {
    auth::require_admin(&state, &headers).await?;
    if spec.id != id || !spec.valid() {
        return Err(ApiError::BadRequest("拨测配置无效".into()));
    }
    let result = sqlx::query("UPDATE network_probes SET spec=$3 WHERE server_id=$1 AND id=$2")
        .bind(server)
        .bind(id)
        .bind(serde_json::to_value(&spec).map_err(anyhow::Error::from)?)
        .execute(&state.pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }
    Ok(Json(spec))
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server, id)): Path<(i64, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    auth::require_admin(&state, &headers).await?;
    if sqlx::query("DELETE FROM network_probes WHERE server_id=$1 AND id=$2")
        .bind(server)
        .bind(id)
        .execute(&state.pool)
        .await?
        .rows_affected()
        == 0
    {
        return Err(ApiError::NotFound);
    }
    Ok(Json(serde_json::json!({"deleted":true})))
}

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryQuery {
    probe_id: Option<Uuid>,
    hours: Option<u32>,
}

pub async fn history(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server): Path<i64>,
    Query(query): Query<HistoryQuery>,
) -> ApiResult<Json<Vec<ProbeResult>>> {
    auth::require_admin(&state, &headers).await?;
    let hours = query.hours.unwrap_or(24);
    if !(1..=24).contains(&hours) {
        return Err(ApiError::BadRequest("拨测历史范围须为 1 至 24 小时".into()));
    }
    let now = now_millis();
    let since = now - i64::from(hours) * 3_600_000;
    let rows: Vec<serde_json::Value> = if let Some(probe) = query.probe_id {
        // A target sampled every ten seconds needs 8640 rows for a complete day.
        sqlx::query_scalar("SELECT result FROM probe_results WHERE server_id=$1 AND probe_id=$2 AND sampled_at>$3 AND sampled_at<=$4 ORDER BY sampled_at DESC,id DESC LIMIT 8641")
            .bind(server).bind(probe).bind(since).bind(now).fetch_all(&state.pool).await?
    } else {
        sqlx::query_scalar("SELECT result FROM probe_results WHERE server_id=$1 AND sampled_at>$2 AND sampled_at<=$3 ORDER BY sampled_at DESC,id DESC LIMIT 4096")
            .bind(server).bind(since).bind(now).fetch_all(&state.pool).await?
    };
    Ok(Json(
        rows.into_iter()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()
            .map_err(anyhow::Error::from)?,
    ))
}

pub async fn ingest(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(batch): Json<ProbeBatch>,
) -> ApiResult<Json<TaskAck>> {
    let server = auth::require_agent(&state, &headers).await?;
    let now = now_millis();
    let mut ids = std::collections::HashSet::new();
    if batch.results.is_empty()
        || batch.results.len() > 64
        || batch.results.iter().any(|r| {
            !ids.insert(r.id)
                || r.sampled_at > now + 60_000
                || r.sampled_at < now - 7 * 86_400_000
                || !r.loss_percent.is_finite()
                || !(0.0..=100.0).contains(&r.loss_percent)
                || r.latency_ms
                    .is_some_and(|v| !v.is_finite() || !(0.0..=60_000.0).contains(&v))
                || r.error.as_ref().is_some_and(|v| v.len() > 1024)
        })
    {
        return Err(ApiError::BadRequest("拨测结果格式无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SELECT id FROM servers WHERE id=$1 FOR UPDATE")
        .bind(server)
        .fetch_one(&mut *tx)
        .await?;
    let mut acknowledged = Vec::new();
    for result in batch.results {
        let digest = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&result).map_err(anyhow::Error::from)?)
        );
        let previous: Option<(i64, String)> =
            sqlx::query_as("SELECT server_id,digest FROM probe_results WHERE id=$1")
                .bind(result.id)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some((owner, saved)) = previous {
            if owner != server || digest != saved {
                return Err(ApiError::Conflict("拨测结果标识已存在不同内容".into()));
            }
        } else {
            let belongs: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM network_probes WHERE id=$1 AND server_id=$2)",
            )
            .bind(result.probe_id)
            .bind(server)
            .fetch_one(&mut *tx)
            .await?;
            // Removed probes acknowledge and discard late samples from an offline Agent.
            if belongs {
                sqlx::query("INSERT INTO probe_results(id,server_id,probe_id,sampled_at,result,digest) VALUES($1,$2,$3,$4,$5,$6)").bind(result.id).bind(server).bind(result.probe_id).bind(result.sampled_at).bind(serde_json::to_value(&result).map_err(anyhow::Error::from)?).bind(digest).execute(&mut *tx).await?;
            }
        }
        acknowledged.push(result.id);
    }
    sqlx::query("DELETE FROM probe_results WHERE server_id=$1 AND sampled_at<$2")
        .bind(server)
        .bind(now - 7 * 86_400_000)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(TaskAck { ids: acknowledged }))
}
