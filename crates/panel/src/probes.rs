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
use sinan_protocol::{
    ProbeBatch, ProbeResult, ProbeSpec, TaskAck, now_timestamp, telemetry::now_millis,
};
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct Overview {
    pub(crate) server_id: i64,
    pub(crate) probe: ProbeSpec,
    pub(crate) results: Vec<ProbeResult>,
}

pub async fn overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<Overview>>> {
    auth::require_admin(&state, &headers).await?;
    read_overview(&state, false).await
}

pub(crate) async fn read_overview(
    state: &AppState,
    visible_only: bool,
) -> ApiResult<Json<Vec<Overview>>> {
    // One indexed query for all cards; each target has an independent sample budget.
    let rows: Vec<(i64, serde_json::Value, serde_json::Value)> = sqlx::query_as(
        "SELECT p.server_id, p.spec, COALESCE(samples.results, '[]'::jsonb)
         FROM network_probes p JOIN servers s ON s.id=p.server_id AND s.deleted_at IS NULL
         LEFT JOIN LATERAL (
             SELECT jsonb_agg(recent.result ORDER BY recent.sampled_at DESC, recent.id DESC) AS results
             FROM (SELECT id, sampled_at, result FROM probe_results
                   WHERE server_id=p.server_id AND probe_id=p.id AND sampled_at>$1 AND sampled_at<=$2
                   ORDER BY sampled_at DESC, id DESC LIMIT 20) recent
         ) samples ON TRUE WHERE (NOT $3 OR COALESCE(s.asset_settings->>'hidden','false')<>'true') ORDER BY p.server_id, p.id",
    )
    .bind(now_millis() - 86_400_000)
    .bind(now_millis())
    .bind(visible_only)
    .fetch_all(&state.pool).await?;
    Ok(Json(
        rows.into_iter()
            .map(|(server_id, spec, results)| {
                Ok(Overview {
                    server_id,
                    probe: presentation(serde_json::from_value(spec)?),
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
) -> ApiResult<Json<Vec<serde_json::Value>>> {
    auth::require_admin(&state, &headers).await?;
    let rows: Vec<(serde_json::Value, Option<Uuid>)> =
        sqlx::query_as("SELECT spec,task_id FROM network_probes WHERE server_id=$1 ORDER BY id")
            .bind(server)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(mut spec, task)| {
                spec["execution_authorized"] = serde_json::json!(
                    serde_json::from_value::<ProbeSpec>(spec.clone())
                        .is_ok_and(|spec| spec.authorized_at(now_timestamp()))
                );
                if let Some(task) = task {
                    spec["task_id"] = serde_json::json!(task);
                }
                spec
            })
            .collect(),
    ))
}

pub(crate) async fn read(state: &AppState, server: i64) -> ApiResult<Json<Vec<ProbeSpec>>> {
    let rows: Vec<serde_json::Value> =
        sqlx::query_scalar("SELECT spec FROM network_probes WHERE server_id=$1 ORDER BY id")
            .bind(server)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(
        rows.into_iter()
            .map(|value| serde_json::from_value(value).map(presentation))
            .collect::<Result<_, _>>()
            .map_err(anyhow::Error::from)?,
    ))
}

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentProbeQuery {
    authorization: Option<u8>,
}

pub(crate) fn presentation(mut spec: ProbeSpec) -> ProbeSpec {
    spec.execution_authorized = Some(spec.authorized_at(now_timestamp()));
    spec
}

pub(crate) fn prepare_write(spec: &mut ProbeSpec) -> ApiResult<()> {
    spec.execution_authorized = None;
    spec.name = spec.name.trim().into();
    spec.target = spec.target.trim().into();
    spec.carrier = spec.carrier.trim().into();
    if let Some(monitor) = &mut spec.monitor {
        monitor.region = monitor.region.trim().into();
        if let Some(authorization) = &mut monitor.authorization {
            authorization.source = authorization.source.trim().into();
            authorization.scope = authorization.scope.trim().into();
        }
    }
    if !spec.valid() || (spec.enabled && !spec.authorized_at(now_timestamp())) {
        return Err(ApiError::BadRequest(
            "启用拨测须登记自有或明确获准的目标、授权来源及范围，且授权未撤销或过期".into(),
        ));
    }
    Ok(())
}

pub async fn agent_list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<AgentProbeQuery>,
) -> ApiResult<Json<Vec<ProbeSpec>>> {
    let server = auth::require_agent(&state, &headers).await?;
    if query.authorization.is_some_and(|version| version != 1) {
        return Err(ApiError::BadRequest("拨测授权协议版本不支持".into()));
    }
    let mut specs = read(&state, server).await?;
    for spec in &mut specs.0 {
        spec.enabled = spec.runnable_at(now_timestamp());
        spec.execution_authorized = None;
        if query.authorization != Some(1) {
            // Old executors cannot enforce cached authorization expiry or address family.
            spec.enabled = false;
            spec.monitor = None;
        }
    }
    Ok(specs)
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server): Path<i64>,
    Json(mut spec): Json<ProbeSpec>,
) -> ApiResult<Json<ProbeSpec>> {
    auth::require_admin(&state, &headers).await?;
    spec.id = Uuid::new_v4();
    prepare_write(&mut spec)?;
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
    Ok(Json(presentation(spec)))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server, id)): Path<(i64, Uuid)>,
    Json(mut spec): Json<ProbeSpec>,
) -> ApiResult<Json<ProbeSpec>> {
    auth::require_admin(&state, &headers).await?;
    if spec.id != id {
        return Err(ApiError::BadRequest("拨测配置无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    let (previous, task): (serde_json::Value, Option<Uuid>) = sqlx::query_as(
        "SELECT spec,task_id FROM network_probes WHERE server_id=$1 AND id=$2 FOR UPDATE",
    )
    .bind(server)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    if task.is_some() {
        return Err(ApiError::Conflict(
            "此目标由统一延迟任务管理，请在延迟检测页面修改".into(),
        ));
    }
    let previous: ProbeSpec = serde_json::from_value(previous).map_err(anyhow::Error::from)?;
    // Samples and offline retries identify their destination only by this immutable ID.
    if !spec.same_measurement_identity(&previous) {
        return Err(ApiError::Conflict(
            "拨测方式、目标、端口、网络版本、运营商和地区创建后不可修改；请新建目标以保留历史归属"
                .into(),
        ));
    }
    prepare_write(&mut spec)?;
    sqlx::query("UPDATE network_probes SET spec=$3 WHERE server_id=$1 AND id=$2")
        .bind(server)
        .bind(id)
        .bind(serde_json::to_value(&spec).map_err(anyhow::Error::from)?)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(presentation(spec)))
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server, id)): Path<(i64, Uuid)>,
) -> ApiResult<Json<serde_json::Value>> {
    auth::require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    let (task,): (Option<Uuid>,) = sqlx::query_as(
        "SELECT task_id FROM network_probes WHERE server_id=$1 AND id=$2 FOR UPDATE",
    )
    .bind(server)
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    if task.is_some() {
        return Err(ApiError::Conflict(
            "此目标由统一延迟任务管理，请在延迟检测页面取消分配".into(),
        ));
    }
    if sqlx::query("DELETE FROM network_probes WHERE server_id=$1 AND id=$2")
        .bind(server)
        .bind(id)
        .execute(&mut *tx)
        .await?
        .rows_affected()
        == 0
    {
        return Err(ApiError::NotFound);
    }
    tx.commit().await?;
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
    read_history(&state, server, query).await
}

pub(crate) async fn read_history(
    state: &AppState,
    server: i64,
    query: HistoryQuery,
) -> ApiResult<Json<Vec<ProbeResult>>> {
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
                || r.address_family == Some(sinan_protocol::ProbeAddressFamily::Any)
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
            let configured: Option<serde_json::Value> =
                sqlx::query_scalar("SELECT spec FROM network_probes WHERE id=$1 AND server_id=$2")
                    .bind(result.probe_id)
                    .bind(server)
                    .fetch_optional(&mut *tx)
                    .await?;
            // Removed or revoked targets discard late samples; prior accepted history remains.
            let authorized = configured
                .and_then(|value| serde_json::from_value::<ProbeSpec>(value).ok())
                .is_some_and(|spec| {
                    spec.authorized_at(result.sampled_at.div_euclid(1000))
                        && result.address_family.is_none_or(|family| {
                            spec.address_family() == sinan_protocol::ProbeAddressFamily::Any
                                || spec.address_family() == family
                        })
                });
            if authorized {
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
