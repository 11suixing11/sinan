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
    let Ok(mut directories) =
        tokio::fs::read_dir(state.config.data_dir.join("artifacts/agent")).await
    else {
        return Ok(Json(None));
    };
    let mut versions = Vec::new();
    while let Some(entry) = directories
        .next_entry()
        .await
        .map_err(anyhow::Error::from)?
    {
        let version = entry.file_name().to_string_lossy().into_owned();
        if let Some(key) = release_version(&version)
            && key > current
        {
            versions.push((key, version));
        }
    }
    versions.sort_by_key(|a| std::cmp::Reverse(a.0));
    for (_, version) in versions {
        if let Ok(artifact) = artifacts::descriptor(&state, "agent", &version, &target).await {
            return Ok(Json(Some(AgentRelease { version, artifact })));
        }
    }
    Ok(Json(None))
}
