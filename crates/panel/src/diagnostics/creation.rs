use super::*;

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Json(request): Json<ReportRequest>,
) -> ApiResult<(StatusCode, Json<ReportRecord>)> {
    auth::require_admin(&state, &headers).await?;
    modes::validate_request(&request)?;
    if !matches!(request.ip_version.as_str(), "both" | "ipv4" | "ipv6")
        || !matches!(request.network_mode.as_str(), "low" | "normal")
    {
        return Err(ApiError::BadRequest("IP 版本或网络测试模式无效".into()));
    }
    let mut tx = state.pool.begin().await?;
    let row = sqlx::query("SELECT static_info,last_seen,capabilities FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE")
        .bind(id).fetch_optional(&mut *tx).await?.ok_or(ApiError::NotFound)?;
    let arch = ready(&row)?;
    let activity = modes::activity_on(&mut tx, id).await?;
    if request.mode == "full"
        && activity.state != "not_enabled"
        && !request.acknowledge_traffic_warning
    {
        return Err(ApiError::Conflict(format!(
            "{} 必须明确确认此警告后继续完整验机。",
            activity.reason
        )));
    }
    let now = now_timestamp();
    sqlx::query("UPDATE diagnostic_jobs SET status='failed',error='任务超时或设备未及时回报',updated_at=$2 WHERE server_id=$1 AND status IN ('queued','running') AND expires_at<=$2")
        .bind(id).bind(now).execute(&mut *tx).await?;
    let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM diagnostic_jobs WHERE server_id=$1 AND status IN ('queued','running'))")
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
    let daily = request.mode == "daily";
    let confirmation = serde_json::json!({
        "confirmed_full": request.confirm_full,
        "acknowledged_traffic_warning": request.acknowledge_traffic_warning,
        "confirmed_at": now,
    });
    let timeout_secs = if daily { 90 } else { TIMEOUT_SECS };
    let mut options = BTreeMap::from([
        ("mode".into(), request.mode),
        ("environment_section".into(), "true".into()),
        ("ip_version".into(), request.ip_version),
        ("network_mode".into(), request.network_mode),
        ("upload_report".into(), request.upload_report.to_string()),
    ]);
    if daily {
        options.insert(
            "daily_targets".into(),
            modes::daily_targets(&mut tx, id).await?,
        );
    }
    let expected = if daily {
        vec!["net_quality", "environment"]
    } else {
        EXPECTED_SECTIONS
            .into_iter()
            .chain(["environment"])
            .collect()
    };
    let job = DiagnosticJob {
        id: Uuid::new_v4(),
        plugin: "nodequality".into(),
        version: PLUGIN_VERSION.into(),
        artifact,
        timeout_secs,
        expires_at: Some(now + timeout_secs as i64 + 300),
        options,
    };
    let mut saved_job = serde_json::to_value(&job).map_err(anyhow::Error::from)?;
    saved_job["proxy_activity"] = serde_json::to_value(activity).map_err(anyhow::Error::from)?;
    saved_job["confirmation"] = confirmation;
    let record = sqlx::query_as("INSERT INTO diagnostic_jobs(id,server_id,job,created_at,updated_at,expires_at,expected_sections) VALUES($1,$2,$3,$4,$4,$5,$6) RETURNING id,status,job,report,error,created_at,updated_at,expires_at,expected_sections,report_completeness,'[]'::jsonb AS sections")
        .bind(job.id).bind(id).bind(saved_job)
        .bind(now).bind(now + timeout_secs as i64 + 300).bind(expected).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(record)))
}
