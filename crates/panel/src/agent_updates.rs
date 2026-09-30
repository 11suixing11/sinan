use crate::{
    AppState, artifacts, auth,
    error::{ApiError, ApiResult},
};
use axum::{Json, extract::State, http::HeaderMap};
use sinan_protocol::{
    AgentRelease, AgentSettings, StaticInfo, platform::artifact_target, release_version,
};
use sqlx::Row;

pub async fn available(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Option<AgentRelease>>> {
    let server = auth::require_agent(&state, &headers).await?;
    let row = sqlx::query(
        "SELECT agent_settings,static_info FROM servers WHERE id=$1 AND deleted_at IS NULL",
    )
    .bind(server)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)?;
    let settings: AgentSettings =
        serde_json::from_value(row.get("agent_settings")).map_err(anyhow::Error::from)?;
    if !settings.auto_update {
        return Ok(Json(None));
    }
    let info: StaticInfo =
        serde_json::from_value(row.get("static_info")).map_err(anyhow::Error::from)?;
    let Some(target) = info
        .os
        .as_deref()
        .zip(info.arch.as_deref())
        .and_then(|(os, arch)| artifact_target(os, info.libc.as_deref(), arch))
    else {
        return Ok(Json(None));
    };
    let current = info
        .agent_version
        .as_deref()
        .and_then(release_version)
        .unwrap_or((0, 0, 0));
    artifacts::require_signed_agent(&state, server).await?;
    let mut targets = vec![target];
    // Older signed Linux releases store static Agents under architecture-only keys.
    if info.os.as_deref() == Some("linux") && info.libc.as_deref() == Some("musl") {
        let arch = match info.arch.as_deref() {
            Some("x86_64" | "amd64") => "amd64",
            Some("aarch64" | "arm64") => "arm64",
            _ => return Ok(Json(None)),
        };
        targets.push(arch.into());
    }
    Ok(Json(
        crate::releases::newer_agent(&state, &targets, current).await?,
    ))
}
