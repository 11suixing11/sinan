use crate::{
    AppState, artifacts, auth,
    error::{ApiError, ApiResult},
    ip_quality::{self, ServerIpInfoView},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sinan_protocol::{
    DiagnosticJob, DiagnosticReport, DiagnosticSectionUpdate, DiagnosticStatus, DiagnosticUpdate,
    now_timestamp,
};
use sqlx::{FromRow, Row};
use std::collections::BTreeMap;
use uuid::Uuid;

pub const PLUGIN_VERSION: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545-r4";
pub const REPORT_LIMIT: usize = 512 * 1024;
const TIMEOUT_SECS: u64 = 1800;
const EXPECTED_SECTIONS: [&str; 5] = [
    "header_info",
    "hardware_quality",
    "ip_quality",
    "net_quality",
    "backroute_trace",
];
mod creation;
mod modes;
pub use creation::create;
mod sections;
const HISTORY_QUERY: &str = "SELECT j.id,j.status,j.job,j.report,j.error,j.created_at,j.updated_at,j.expires_at,j.expected_sections,j.report_completeness,COALESCE((SELECT jsonb_agg(jsonb_build_object('name',s.name,'text',s.text,'complete',s.complete,'revision',s.revision,'collected_at',s.collected_at) ORDER BY array_position(j.expected_sections,s.name),s.name) FROM diagnostic_report_sections s WHERE s.job_id=j.id),'[]'::jsonb) AS sections FROM diagnostic_jobs j WHERE j.server_id=$1 ORDER BY j.created_at DESC,j.id DESC LIMIT 10";

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
    pub expected_sections: Vec<String>,
    pub report_completeness: String,
    pub sections: Value,
}

#[derive(Serialize)]
pub struct NodeQualityView {
    pub plugin_ready: bool,
    pub plugin_reason: Option<String>,
    pub reports: Vec<ReportRecord>,
    pub proxy_activity: modes::ProxyActivity,
}

#[derive(Serialize)]
pub struct LegacyNodeQualityView {
    #[serde(flatten)]
    pub node_quality: NodeQualityView,
    #[serde(flatten)]
    pub ip_info: ServerIpInfoView,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportRequest {
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default)]
    pub confirm_full: bool,
    #[serde(default)]
    pub acknowledge_traffic_warning: bool,
    #[serde(default = "default_ip_version")]
    pub ip_version: String,
    #[serde(default = "default_network_mode")]
    pub network_mode: String,
    #[serde(default)]
    pub upload_report: bool,
}

fn default_mode() -> String {
    "full".into()
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
    let info: Value = row.get("static_info");
    if info["os"].as_str() != Some("linux") {
        return Err(ApiError::Conflict(
            "当前诊断插件仅支持已识别的 Linux 设备".into(),
        ));
    }
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
        values
            .iter()
            .any(|value| value.as_str() == Some("diagnostic:nodequality-modes"))
    }) {
        return Err(ApiError::Conflict(
            "此 Agent 尚不支持日常检查与完整验机入口，请先升级 Agent".into(),
        ));
    }
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
    if !capabilities.as_array().is_some_and(|values| {
        values
            .iter()
            .any(|value| value.as_str() == Some(sinan_protocol::DIAGNOSTIC_SECTIONS_CAPABILITY))
    }) {
        return Err(ApiError::Conflict(
            "此 Agent 尚不支持独立报告章节，请先升级 Agent".into(),
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
    Ok(sqlx::query_as(HISTORY_QUERY)
        .bind(server_id)
        .fetch_all(&state.pool)
        .await?)
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<NodeQualityView>> {
    auth::require_admin(&state, &headers).await?;
    Ok(Json(view(&state, id).await?))
}

pub async fn legacy_get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<LegacyNodeQualityView>> {
    auth::require_admin(&state, &headers).await?;
    Ok(Json(LegacyNodeQualityView {
        node_quality: view(&state, id).await?,
        ip_info: ip_quality::view(&state, id).await?,
    }))
}

async fn view(state: &AppState, id: i64) -> ApiResult<NodeQualityView> {
    expire(state).await?;
    let row = sqlx::query(
        "SELECT static_info,last_seen,capabilities FROM servers WHERE id=$1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let mut reason = ready(&row).err().map(|error| error.to_string());
    if reason.is_none() {
        let arch = ready(&row)?;
        if let Err(error) = artifacts::descriptor(state, "nodequality", PLUGIN_VERSION, arch).await
        {
            reason = Some(match error {
                ApiError::NotFound => {
                    "NodeQuality 插件制品尚未上传，请先准备对应架构的制品及 SHA256SUMS".into()
                }
                other => format!("NodeQuality 插件制品不可用：{other}"),
            });
        }
    }
    Ok(NodeQualityView {
        plugin_ready: reason.is_none(),
        plugin_reason: reason,
        reports: history(state, id).await?,
        proxy_activity: modes::activity(&state.pool, id).await?,
    })
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
    if matches!(status.as_str(), "succeeded" | "failed")
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
    .bind(&report)
    .bind(error)
    .bind(now)
    .bind(completed)
    .execute(&mut *tx)
    .await?;
    if report.is_some() {
        sqlx::query("UPDATE diagnostic_jobs SET report_completeness='legacy' WHERE id=$1 AND cardinality(expected_sections)=0")
            .bind(id).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub use sections::upload_section;

#[cfg(test)]
mod tests {
    use super::{ReportRequest, safe_report_url};
    use serde_json::json;

    #[test]
    fn report_upload_requires_an_explicit_boolean_opt_in() {
        for request in [json!({}), json!({"upload_report":false})] {
            let request: ReportRequest = serde_json::from_value(request).unwrap();
            assert!(!request.upload_report);
        }
        let request: ReportRequest = serde_json::from_value(json!({"upload_report":true})).unwrap();
        assert!(request.upload_report);
        for value in [json!("true"), json!(1), json!(null)] {
            assert!(
                serde_json::from_value::<ReportRequest>(json!({"upload_report":value})).is_err()
            );
        }
    }
    #[test]
    fn report_links_are_restricted_to_the_official_https_origin() {
        assert!(safe_report_url("https://nodequality.com/r/example"));
        for url in [
            "http://nodequality.com/r/example",
            "https://nodequality.com.evil.invalid/r/x",
            "https://user:secret@nodequality.com/r/x",
            "https://nodequality.com:8443/r/x",
            "javascript:alert(1)",
            "https://nodequality.com/r/x#fragment",
        ] {
            assert!(!safe_report_url(url), "{url}");
        }
    }
}
