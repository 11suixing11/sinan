use crate::{
    AppState, artifacts, auth,
    error::{ApiError, ApiResult},
    ip_quality::{self, IpQuality},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sinan_protocol::{
    DiagnosticJob, DiagnosticReport, DiagnosticStatus, DiagnosticUpdate, now_timestamp,
};
use sqlx::{FromRow, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const PLUGIN_VERSION: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r2";
pub const REPORT_LIMIT: usize = 512 * 1024;
const TIMEOUT_SECS: u64 = 1800;

pub mod cancellation;

#[derive(Serialize, FromRow)]
pub struct ReportRecord {
    pub id: Uuid,
    pub status: String,
    pub job: Value,
    pub report: Option<Value>,
    pub error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: i64,
    pub agent_completed: bool,
    pub cancel_requested_at: Option<i64>,
    pub cancel_error: Option<String>,
}

#[derive(Serialize)]
pub struct NodeQualityView {
    pub ip_addresses: Vec<String>,
    pub quality: Vec<IpQuality>,
    pub plugin_ready: bool,
    pub plugin_reason: Option<String>,
    pub reports: Vec<ReportRecord>,
    pub cancel_supported: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportRequest {
    #[serde(default = "default_ip_version")]
    pub ip_version: String,
    #[serde(default = "default_network_mode")]
    pub network_mode: String,
    #[serde(default)]
    pub upload_report: bool,
}

fn default_ip_version() -> String {
    "both".into()
}
fn default_network_mode() -> String {
    "low".into()
}

fn architecture(info: &Value) -> ApiResult<&'static str> {
    match info["arch"].as_str() {
        Some("aarch64" | "arm64") => Ok("arm64"),
        Some("x86_64" | "amd64") => Ok("amd64"),
        _ => Err(ApiError::Conflict("设备架构未知，无法运行诊断插件".into())),
    }
}

fn ready(row: &sqlx::postgres::PgRow) -> ApiResult<&'static str> {
    let online = row
        .get::<Option<i64>, _>("last_seen")
        .is_some_and(|seen| now_timestamp().saturating_sub(seen) <= 60);
    if !online {
        return Err(ApiError::Conflict(
            "Agent 当前离线，请在设备上线后运行报告".into(),
        ));
    }
    let capabilities: Value = row.get("capabilities");
    if !capabilities.as_array().is_some_and(|values| {
        values.iter().any(|value| {
            value.as_str() == Some(sinan_protocol::release::ARTIFACT_SIGNATURE_CAPABILITY)
        })
    }) {
        return Err(ApiError::Conflict(
            "此 Agent 尚不支持制品验签，请先升级 Agent".into(),
        ));
    }
    if !capabilities.as_array().is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str() == Some("diagnostic:nodequality"))
    }) {
        return Err(ApiError::Conflict(
            "此 Agent 尚不支持 NodeQuality 插件，请升级 Agent".into(),
        ));
    }
    architecture(&row.get::<Value, _>("static_info"))
}

pub async fn expire(state: &AppState) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE diagnostic_jobs SET status='failed',error='任务超时或设备未及时回报，请检查设备后重新运行',updated_at=$1 WHERE status IN ('queued','running') AND expires_at<=$1")
        .bind(now_timestamp()).execute(&state.pool).await?;
    Ok(())
}

async fn history(state: &AppState, server_id: i64) -> ApiResult<Vec<ReportRecord>> {
    Ok(sqlx::query_as("SELECT id,status,job,report,error,created_at,updated_at,expires_at,agent_completed,cancel_requested_at,cancel_error FROM diagnostic_jobs WHERE server_id=$1 ORDER BY created_at DESC,id DESC LIMIT 10")
        .bind(server_id).fetch_all(&state.pool).await?)
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<NodeQualityView>> {
    auth::require_admin(&state, &headers).await?;
    expire(&state).await?;
    let row = sqlx::query(
        "SELECT static_info,last_seen,capabilities FROM servers WHERE id=$1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let ips = ip_quality::reported_ips(&row.get::<Value, _>("static_info"));
    let mut reason = ready(&row).err().map(|error| error.to_string());
    if reason.is_none() {
        let arch = ready(&row)?;
        if let Err(error) = artifacts::descriptor(&state, "nodequality", PLUGIN_VERSION, arch).await
        {
            reason = Some(match error {
                ApiError::NotFound => {
                    "NodeQuality 插件制品尚未上传，请先准备对应架构的制品及 SHA256SUMS".into()
                }
                other => format!("NodeQuality 插件制品不可用：{other}"),
            });
        }
    }
    Ok(Json(NodeQualityView {
        quality: ip_quality::cached(&state, id, &ips).await?,
        ip_addresses: ips,
        plugin_ready: reason.is_none(),
        plugin_reason: reason,
        reports: history(&state, id).await?,
        cancel_supported: cancellation::supported(&row.get::<Value, _>("capabilities")),
    }))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<ReportRequest>,
) -> ApiResult<(StatusCode, Json<ReportRecord>)> {
    auth::require_admin(&state, &headers).await?;
    if !matches!(request.ip_version.as_str(), "both" | "ipv4" | "ipv6")
        || !matches!(request.network_mode.as_str(), "low" | "normal")
    {
        return Err(ApiError::BadRequest("IP 版本或网络测试模式无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query("SELECT static_info,last_seen,capabilities FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
        .bind(id).fetch_optional(&mut *tx).await?.ok_or(ApiError::NotFound)?;
    let arch = ready(&row)?;
    let now = now_timestamp();
    sqlx::query("UPDATE diagnostic_jobs SET status='failed',error='任务超时或设备未及时回报',updated_at=$2 WHERE server_id=$1 AND status IN ('queued','running') AND expires_at<=$2")
        .bind(id).bind(now).execute(&mut *tx).await?;
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM diagnostic_jobs WHERE server_id=$1 AND status IN ('queued','running','cancel_requested'))")
        .bind(id).fetch_one(&mut *tx).await?;
    if active {
        return Err(ApiError::Conflict(
            "此服务器已有正在执行的报告任务，请等待完成".into(),
        ));
    }
    let artifact = artifacts::descriptor(&state, "nodequality", PLUGIN_VERSION, arch)
        .await
        .map_err(|error| match error {
            ApiError::NotFound => ApiError::Conflict(
                "NodeQuality 插件制品尚未上传，请先准备对应架构的制品及 SHA256SUMS".into(),
            ),
            error => error,
        })?;
    let job = DiagnosticJob {
        id: Uuid::new_v4(),
        plugin: "nodequality".into(),
        version: PLUGIN_VERSION.into(),
        artifact,
        timeout_secs: TIMEOUT_SECS,
        expires_at: Some(now + TIMEOUT_SECS as i64 + 300),
        options: BTreeMap::from([
            ("ip_version".into(), request.ip_version),
            ("network_mode".into(), request.network_mode),
            ("upload_report".into(), request.upload_report.to_string()),
        ]),
    };
    let record = sqlx::query_as("INSERT INTO diagnostic_jobs(id,server_id,job,created_at,updated_at,expires_at) VALUES($1,$2,$3,$4,$4,$5) RETURNING id,status,job,report,error,created_at,updated_at,expires_at,agent_completed,cancel_requested_at,cancel_error")
        .bind(job.id).bind(id).bind(serde_json::to_value(job).map_err(anyhow::Error::from)?)
        .bind(now).bind(now + TIMEOUT_SECS as i64 + 300).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(record)))
}

pub async fn pending(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<DiagnosticJob>>> {
    let server_id = auth::require_agent(&state, &headers).await?;
    artifacts::require_signed_agent(&state, server_id).await?;
    expire(&state).await?;
    let values: Vec<Value> = sqlx::query_scalar("SELECT job FROM diagnostic_jobs WHERE server_id=$1 AND status IN ('queued','running') ORDER BY created_at,id")
        .bind(server_id).fetch_all(&state.pool).await?;
    let jobs = values
        .into_iter()
        .map(serde_json::from_value)
        .collect::<Result<_, _>>()
        .map_err(anyhow::Error::from)?;
    Ok(Json(jobs))
}

fn validate_report(report: &DiagnosticReport) -> ApiResult<()> {
    if report.text.trim().is_empty() || report.text.len() > REPORT_LIMIT {
        return Err(ApiError::BadRequest("报告文本为空或超过 512 KiB".into()));
    }
    if report
        .report_url
        .as_ref()
        .is_some_and(|url| !safe_report_url(url))
    {
        return Err(ApiError::BadRequest(
            "报告链接必须是 NodeQuality 官方 HTTPS 地址".into(),
        ));
    }
    Ok(())
}

pub fn safe_report_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    value.len() <= 2048
        && url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some("nodequality.com" | "www.nodequality.com")
        )
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.fragment().is_none()
}

pub async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(update): Json<DiagnosticUpdate>,
) -> ApiResult<StatusCode> {
    let server_id = auth::require_agent(&state, &headers).await?;
    if id != update.id {
        return Err(ApiError::BadRequest("任务编号不一致".into()));
    }
    if let Some(report) = &update.report {
        validate_report(report)?;
    }
    if update.status == DiagnosticStatus::Succeeded && update.report.is_none() {
        return Err(ApiError::BadRequest("成功任务必须包含完整报告文本".into()));
    }
    if update.status == DiagnosticStatus::Running
        && (update.report.is_some() || update.error.is_some())
    {
        return Err(ApiError::BadRequest("进行中的任务不能回报最终结果".into()));
    }
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query(
        "SELECT status,expires_at,agent_completed FROM diagnostic_jobs WHERE id=$1 AND server_id=$2 FOR UPDATE",
    )
    .bind(id)
    .bind(server_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    let status: String = row.get("status");
    if status == "cancel_requested" {
        // A late natural result cannot confirm that cancellation has cleaned up.
        let report = update
            .report
            .map(serde_json::to_value)
            .transpose()
            .map_err(anyhow::Error::from)?;
        sqlx::query("UPDATE diagnostic_jobs SET report=COALESCE($2,report),agent_completed=agent_completed OR $3,updated_at=$4 WHERE id=$1")
            .bind(id).bind(report).bind(update.status != DiagnosticStatus::Running).bind(now_timestamp())
            .execute(&mut *tx).await?;
        tx.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }

    if matches!(status.as_str(), "succeeded" | "failed" | "cancelled")
        && (row.get::<bool, _>("agent_completed") || update.status == DiagnosticStatus::Running)
    {
        // Final updates are retried durably by devices; terminal results never regress.
        tx.commit().await?;
        return Ok(StatusCode::NO_CONTENT);
    }
    let now = now_timestamp();
    let completed = update.status != DiagnosticStatus::Running;
    let (status, error) = if row.get::<i64, _>("expires_at") <= now && !completed {
        ("failed", Some("任务超时，迟到回报已忽略".to_owned()))
    } else {
        let status = match update.status {
            DiagnosticStatus::Running => "running",
            DiagnosticStatus::Succeeded => "succeeded",
            DiagnosticStatus::Failed => "failed",
        };
        let error = update.error.map(|value| value.chars().take(4096).collect());
        (
            status,
            error.or_else(|| {
                (update.status == DiagnosticStatus::Failed)
                    .then(|| "插件执行失败，未提供错误详情".into())
            }),
        )
    };
    let report = update
        .report
        .map(serde_json::to_value)
        .transpose()
        .map_err(anyhow::Error::from)?;
    sqlx::query(
        "UPDATE diagnostic_jobs SET status=$2,report=$3,error=$4,updated_at=$5,agent_completed=$6 WHERE id=$1",
    )
    .bind(id)
    .bind(status)
    .bind(report)
    .bind(error)
    .bind(now)
    .bind(completed)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
