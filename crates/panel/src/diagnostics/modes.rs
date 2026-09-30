use super::*;
use sinan_protocol::{ProbeKind, ProbeSpec};

#[derive(Serialize)]
pub struct ProxyActivity {
    pub state: &'static str,
    pub reason: &'static str,
    pub checked_at: i64,
    pub last_positive_at: Option<i64>,
}

pub async fn activity(pool: &sqlx::PgPool, server: i64) -> ApiResult<ProxyActivity> {
    let mut connection = pool.acquire().await?;
    activity_on(&mut connection, server).await
}

pub async fn activity_on(
    connection: &mut sqlx::PgConnection,
    server: i64,
) -> ApiResult<ProxyActivity> {
    let now = now_timestamp();
    let enabled: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM deployments WHERE server_id=$1 AND module='singbox')",
    )
    .bind(server)
    .fetch_one(&mut *connection)
    .await?;
    let latest: Option<i64> = sqlx::query_scalar("SELECT MAX(period_end) FROM usage_records WHERE server_id=$1 AND (uplink>0 OR downlink>0) AND period_end<=$2")
        .bind(server).bind(now+60).fetch_one(&mut *connection).await?;
    let (state, reason) = if latest.is_some_and(|sampled| sampled >= now - 60) {
        (
            "active",
            "最近一分钟记录到代理流量，完整验机会争抢资源并可能影响连接。",
        )
    } else if enabled {
        // The current ledger stores positive deltas, so silence never proves idle.
        (
            "unknown",
            "代理流量状态未知：没有足够新的计量证据，不能确认当前无活跃连接。",
        )
    } else {
        ("not_enabled", "此服务器没有发布代理运行时配置。")
    };
    Ok(ProxyActivity {
        state,
        reason,
        checked_at: now,
        last_positive_at: latest,
    })
}

pub async fn daily_targets(connection: &mut sqlx::PgConnection, server: i64) -> ApiResult<String> {
    let values: Vec<Value> = sqlx::query_scalar(
        "SELECT spec FROM network_probes WHERE server_id=$1 ORDER BY id LIMIT 32",
    )
    .bind(server)
    .fetch_all(connection)
    .await?;
    let targets: Vec<_> = values
        .into_iter()
        .filter_map(|value| serde_json::from_value::<ProbeSpec>(value).ok())
        .filter(|spec| spec.valid() && spec.enabled && spec.kind == ProbeKind::Tcp)
        .take(4)
        .map(|spec| serde_json::json!({"name":spec.name,"target":spec.target,"port":spec.port}))
        .collect();
    serde_json::to_string(&targets).map_err(|error| anyhow::Error::from(error).into())
}

pub fn validate_request(request: &ReportRequest) -> ApiResult<()> {
    if !matches!(request.mode.as_str(), "daily" | "full") {
        return Err(ApiError::BadRequest("检查入口无效".into()));
    }
    if request.mode == "full" && !request.confirm_full {
        return Err(ApiError::BadRequest(
            "完整验机需要管理员明确确认资源与流量影响".into(),
        ));
    }
    if request.mode == "daily" && (request.network_mode != "low" || request.upload_report) {
        return Err(ApiError::BadRequest(
            "日常检查只允许有限网络探测和本地报告".into(),
        ));
    }
    Ok(())
}
