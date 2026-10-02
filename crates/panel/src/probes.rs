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
    AuthorizedProbe, MAX_PROBE_LEASE_SECS, PROBE_LEASE_CAPABILITY, ProbeAuthorization, ProbeBatch,
    ProbeLease, ProbeResult, ProbeSpec, TaskAck, now_timestamp, telemetry::now_millis,
};
use sqlx::{Postgres, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Clone, serde::Serialize)]
pub struct ConfiguredProbe {
    #[serde(flatten)]
    pub spec: ProbeSpec,
    pub authorization: Option<ProbeAuthorization>,
    pub revision: Option<i64>,
}

impl<'de> serde::Deserialize<'de> for ConfiguredProbe {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut value = serde_json::Value::deserialize(deserializer)?;
        let fields = value
            .as_object_mut()
            .ok_or_else(|| serde::de::Error::custom("probe configuration must be an object"))?;
        let authorization = fields
            .remove("authorization")
            .map(serde_json::from_value::<Option<ProbeAuthorization>>)
            .transpose()
            .map_err(serde::de::Error::custom)?
            .flatten();
        let revision = fields
            .remove("revision")
            .map(serde_json::from_value::<Option<i64>>)
            .transpose()
            .map_err(serde::de::Error::custom)?
            .flatten();
        let spec = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self {
            spec,
            authorization,
            revision,
        })
    }
}

pub(crate) fn validate_authorization(
    enabled: bool,
    authorization: Option<&ProbeAuthorization>,
) -> ApiResult<()> {
    if authorization.is_some_and(|authorization| !authorization.valid()) {
        return Err(ApiError::BadRequest("拨测目标授权信息无效".into()));
    }
    if enabled && !authorization.is_some_and(|authorization| authorization.allows(now_timestamp()))
    {
        return Err(ApiError::BadRequest(
            "启用拨测需要有效的自有目标说明或第三方使用授权；请先填写来源与授权依据".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn bump_revision(
    tx: &mut Transaction<'_, Postgres>,
    server: i64,
) -> ApiResult<()> {
    sqlx::query("UPDATE servers SET probe_revision=probe_revision+1 WHERE id=$1")
        .bind(server)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn lock_server(tx: &mut Transaction<'_, Postgres>, server: i64) -> ApiResult<()> {
    crate::latency_tasks::lock(tx).await?;
    sqlx::query("SELECT id FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
        .bind(server)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(())
}

pub(crate) fn authorized(value: Option<serde_json::Value>, at: i64) -> Option<ProbeAuthorization> {
    value
        .and_then(|value| serde_json::from_value::<ProbeAuthorization>(value).ok())
        .filter(|authorization| authorization.allows(at))
}

fn probe_digest(probe: &AuthorizedProbe) -> ApiResult<String> {
    let canonical = serde_json::to_value(probe).map_err(anyhow::Error::from)?;
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&canonical).map_err(anyhow::Error::from)?)
    ))
}

async fn eligible(
    connection: &mut sqlx::PgConnection,
    server: i64,
    at: i64,
) -> ApiResult<Vec<AuthorizedProbe>> {
    let rows: Vec<(Uuid, serde_json::Value, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT id,spec,target_authorization FROM network_probes WHERE server_id=$1 ORDER BY id LIMIT 33",
    )
    .bind(server)
    .fetch_all(connection)
    .await?;
    if rows.len() > 32 {
        return Err(ApiError::Conflict("拨测目标超过配置上限".into()));
    }
    Ok(rows
        .into_iter()
        .filter_map(|(id, value, authorization)| {
            let spec: ProbeSpec = serde_json::from_value(value).ok()?;
            let authorization = authorized(authorization, at)?;
            (spec.id == id && spec.valid() && spec.enabled).then_some(AuthorizedProbe {
                spec,
                authorization,
            })
        })
        .collect())
}

#[derive(serde::Serialize)]
pub struct Overview {
    pub(crate) server_id: i64,
    pub(crate) probe: ProbeSpec,
    pub(crate) results: Vec<ProbeResult>,
    pub(crate) authorization_state: &'static str,
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
    let rows: Vec<(i64, serde_json::Value, serde_json::Value, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT p.server_id, p.spec, COALESCE(samples.results, '[]'::jsonb), p.target_authorization
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
            .map(|(server_id, spec, results, authorization)| {
                let authorization = authorization
                    .and_then(|value| serde_json::from_value::<ProbeAuthorization>(value).ok());
                let authorization_state = match authorization {
                    Some(authorization) if authorization.allows(now_timestamp()) => "allowed",
                    Some(authorization) if authorization.valid() => "expired",
                    _ => "missing",
                };
                Ok(Overview {
                    server_id,
                    probe: serde_json::from_value(spec)?,
                    results: serde_json::from_value(results)?,
                    authorization_state,
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
    let rows: Vec<(serde_json::Value, Option<Uuid>, Option<serde_json::Value>, i64)> =
        sqlx::query_as("SELECT spec,task_id,target_authorization,revision FROM network_probes WHERE server_id=$1 ORDER BY id")
            .bind(server)
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(
        rows.into_iter()
            .map(|(mut spec, task, authorization, revision)| {
                spec["authorization"] = authorization.unwrap_or(serde_json::Value::Null);
                spec["revision"] = serde_json::json!(revision);
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
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server).await?;
    let probes = eligible(&mut tx, server, now_timestamp())
        .await?
        .into_iter()
        .map(|probe| probe.spec)
        .collect();
    tx.commit().await?;
    Ok(Json(probes))
}

pub async fn agent_lease(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<ProbeLease>> {
    let server = auth::require_agent(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server).await?;
    let (mut revision, capabilities, previous_fingerprint): (
        i64,
        serde_json::Value,
        Option<String>,
    ) = sqlx::query_as(
        "SELECT probe_revision,capabilities,probe_fingerprint FROM servers WHERE id=$1",
    )
    .bind(server)
    .fetch_one(&mut *tx)
    .await?;
    if !capabilities.as_array().is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str() == Some(PROBE_LEASE_CAPABILITY))
    }) {
        return Err(ApiError::Conflict(
            "此 Agent 尚不支持可撤销拨测租约，请先升级 Agent".into(),
        ));
    }
    let issued_at = now_timestamp();
    let probes = eligible(&mut tx, server, issued_at).await?;
    let digests: BTreeMap<_, _> = probes
        .iter()
        .map(|probe| Ok((probe.spec.id.to_string(), probe_digest(probe)?)))
        .collect::<ApiResult<_>>()?;
    let fingerprint = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&digests).map_err(anyhow::Error::from)?)
    );
    if previous_fingerprint.as_deref() != Some(fingerprint.as_str()) {
        revision = sqlx::query_scalar("UPDATE servers SET probe_revision=probe_revision+1,probe_fingerprint=$2 WHERE id=$1 RETURNING probe_revision")
            .bind(server).bind(fingerprint).fetch_one(&mut *tx).await?;
    }
    let saved: Option<(Uuid, i64, i64, serde_json::Value)> = sqlx::query_as(
        "SELECT id,issued_at,expires_at,probe_digests FROM probe_leases WHERE server_id=$1 AND revision=$2 AND issued_at>$3 AND expires_at>$4 ORDER BY issued_at DESC,id DESC LIMIT 1",
    ).bind(server).bind(revision).bind(issued_at - 25).bind(issued_at)
        .fetch_optional(&mut *tx).await?;
    if let Some((id, issued_at, expires_at, saved)) = saved
        && saved == serde_json::to_value(&digests).map_err(anyhow::Error::from)?
    {
        tx.commit().await?;
        return Ok(Json(ProbeLease {
            id,
            server_id: server,
            revision: revision as u64,
            issued_at,
            expires_at,
            probes,
        }));
    }
    let expires_at = probes
        .iter()
        .filter_map(|probe| probe.authorization.expires_at)
        .fold(issued_at + MAX_PROBE_LEASE_SECS, i64::min);
    let lease = ProbeLease {
        id: Uuid::new_v4(),
        server_id: server,
        revision: revision as u64,
        issued_at,
        expires_at,
        probes,
    };
    sqlx::query("INSERT INTO probe_leases(id,server_id,revision,issued_at,expires_at,probe_digests) VALUES($1,$2,$3,$4,$5,$6)")
        .bind(lease.id).bind(server).bind(revision).bind(issued_at).bind(expires_at)
        .bind(serde_json::to_value(digests).map_err(anyhow::Error::from)?)
        .execute(&mut *tx).await?;
    sqlx::query("DELETE FROM probe_leases WHERE server_id=$1 AND (issued_at<$2 OR id IN (SELECT id FROM probe_leases WHERE server_id=$1 ORDER BY issued_at DESC,id DESC OFFSET 512))")
        .bind(server).bind(issued_at - 3 * 3_600).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(lease))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(server): Path<i64>,
    Json(mut input): Json<ConfiguredProbe>,
) -> ApiResult<Json<ConfiguredProbe>> {
    auth::require_admin(&state, &headers).await?;
    let spec = &mut input.spec;
    spec.id = Uuid::new_v4();
    if !spec.valid() {
        return Err(ApiError::BadRequest("拨测配置无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server).await?;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM network_probes WHERE server_id=$1")
        .bind(server)
        .fetch_one(&mut *tx)
        .await?;
    if count >= 32 {
        return Err(ApiError::Conflict("每台服务器最多配置 32 个拨测".into()));
    }
    validate_authorization(spec.enabled, input.authorization.as_ref())?;
    sqlx::query(
        "INSERT INTO network_probes(id,server_id,spec,target_authorization) VALUES($1,$2,$3,$4)",
    )
    .bind(spec.id)
    .bind(server)
    .bind(serde_json::to_value(&spec).map_err(anyhow::Error::from)?)
    .bind(
        input
            .authorization
            .as_ref()
            .map(serde_json::to_value)
            .transpose()
            .map_err(anyhow::Error::from)?,
    )
    .execute(&mut *tx)
    .await?;
    bump_revision(&mut tx, server).await?;
    tx.commit().await?;
    input.revision = Some(1);
    Ok(Json(input))
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server, id)): Path<(i64, Uuid)>,
    Json(mut input): Json<ConfiguredProbe>,
) -> ApiResult<Json<ConfiguredProbe>> {
    auth::require_admin(&state, &headers).await?;
    let spec = &input.spec;
    if spec.id != id || !spec.valid() {
        return Err(ApiError::BadRequest("拨测配置无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server).await?;
    let (previous, task, revision): (serde_json::Value, Option<Uuid>, i64) = sqlx::query_as(
        "SELECT spec,task_id,revision FROM network_probes WHERE server_id=$1 AND id=$2 FOR UPDATE",
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
    if input.revision != Some(revision) {
        return Err(ApiError::Conflict("拨测已被修改，请刷新后重试".into()));
    }
    let previous: ProbeSpec = serde_json::from_value(previous).map_err(anyhow::Error::from)?;
    // Samples and offline retries identify their destination only by this immutable ID.
    if spec.kind != previous.kind || spec.target != previous.target || spec.port != previous.port {
        return Err(ApiError::Conflict(
            "拨测方式、目标地址和端口创建后不可修改；请新建拨测目标以保留历史归属".into(),
        ));
    }
    validate_authorization(spec.enabled, input.authorization.as_ref())?;
    sqlx::query("UPDATE network_probes SET spec=$3,target_authorization=$4,revision=revision+1 WHERE server_id=$1 AND id=$2")
        .bind(server)
        .bind(id)
        .bind(serde_json::to_value(spec).map_err(anyhow::Error::from)?)
        .bind(input.authorization.as_ref().map(serde_json::to_value).transpose().map_err(anyhow::Error::from)?)
        .execute(&mut *tx)
        .await?;
    bump_revision(&mut tx, server).await?;
    tx.commit().await?;
    input.revision = Some(revision + 1);
    Ok(Json(input))
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeleteProbe {
    pub(crate) revision: Option<i64>,
}

pub async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((server, id)): Path<(i64, Uuid)>,
    input: Option<Json<DeleteProbe>>,
) -> ApiResult<Json<serde_json::Value>> {
    auth::require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server).await?;
    let (task, revision): (Option<Uuid>, i64) = sqlx::query_as(
        "SELECT task_id,revision FROM network_probes WHERE server_id=$1 AND id=$2 FOR UPDATE",
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
    if input.and_then(|Json(input)| input.revision) != Some(revision) {
        return Err(ApiError::Conflict("拨测已被修改，请刷新后重试".into()));
    }
    bump_revision(&mut tx, server).await?;
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
        })
    {
        return Err(ApiError::BadRequest("拨测结果格式无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    lock_server(&mut tx, server).await?;
    let (revision, capabilities): (i64, serde_json::Value) =
        sqlx::query_as("SELECT probe_revision,capabilities FROM servers WHERE id=$1")
            .bind(server)
            .fetch_one(&mut *tx)
            .await?;
    let requires_execution = capabilities.as_array().is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str() == Some(PROBE_LEASE_CAPABILITY))
    });
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
            if requires_execution && result.execution.is_none() {
                // Upgraded devices can still have an old unproved outbox. Drain
                // it without inventing permission or blocking proved samples.
                acknowledged.push(result.id);
                continue;
            }
            let current: Option<(serde_json::Value, Option<serde_json::Value>)> = sqlx::query_as(
                "SELECT spec,target_authorization FROM network_probes WHERE id=$1 AND server_id=$2",
            )
            .bind(result.probe_id)
            .bind(server)
            .fetch_optional(&mut *tx)
            .await?;
            let mut belongs = current.is_some();
            if let Some(execution) = &result.execution {
                let within = execution
                    .issued_at
                    .checked_mul(1_000)
                    .zip(execution.expires_at.checked_mul(1_000))
                    .is_some_and(|(start, end)| {
                        result.sampled_at >= start && result.sampled_at < end
                    });
                if !execution.valid() || execution.probe.spec.id != result.probe_id || !within {
                    return Err(ApiError::BadRequest("拨测租约执行上下文无效".into()));
                }
                let receipt: Option<(i64, i64, i64, i64, serde_json::Value)> = sqlx::query_as(
                    "SELECT server_id,revision,issued_at,expires_at,probe_digests FROM probe_leases WHERE id=$1",
                ).bind(execution.lease_id).fetch_optional(&mut *tx).await?;
                let digest = probe_digest(&execution.probe)?;
                let issued =
                    receipt.is_some_and(|(owner, revision, issued_at, expires_at, digests)| {
                        owner == server
                            && revision as u64 == execution.revision
                            && issued_at == execution.issued_at
                            && expires_at == execution.expires_at
                            && issued_at >= now_timestamp() - 3 * 3_600
                            && digests
                                .get(result.probe_id.to_string().as_str())
                                .and_then(serde_json::Value::as_str)
                                == Some(digest.as_str())
                    });
                let current = current.and_then(|(value, authorization)| {
                    Some(AuthorizedProbe {
                        spec: serde_json::from_value(value).ok()?,
                        authorization: authorized(authorization, now_timestamp())?,
                    })
                });
                belongs = issued
                    && revision as u64 == execution.revision
                    && current.as_ref() == Some(&execution.probe);
            }
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
