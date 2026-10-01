use crate::{
    AppState, artifacts,
    error::{ApiError, ApiResult},
};
use serde_json::Value;
use sinan_protocol::ModuleManifest;
use sqlx::Row;

const MANIFEST_ERROR_PREFIX: &str = "清单准备失败：";

async fn preparation_result(
    state: &AppState,
    server_id: i64,
    revision: i64,
    error: Option<&str>,
) -> ApiResult<()> {
    let mut tx = state.pool.begin().await?;
    // Match the publisher's server-before-module order, including FK checks.
    super::business::lock_server(&mut tx, server_id).await?;
    let current: Option<i64> = sqlx::query_scalar("SELECT target_rev FROM server_module_status WHERE server_id=$1 AND module='singbox' FOR UPDATE")
        .bind(server_id).fetch_optional(&mut *tx).await?;
    if current == Some(revision) {
        if let Some(error) = error {
            sqlx::query("INSERT INTO singbox_installation(server_id,target_rev,error,checked_at) VALUES($1,$2,$3,$4) ON CONFLICT(server_id) DO UPDATE SET target_rev=EXCLUDED.target_rev,error=EXCLUDED.error,checked_at=EXCLUDED.checked_at")
                .bind(server_id).bind(revision).bind(error).bind(sinan_protocol::now_timestamp())
                .execute(&mut *tx).await?;
        } else {
            sqlx::query("DELETE FROM singbox_installation WHERE server_id=$1 AND target_rev=$2")
                .bind(server_id)
                .bind(revision)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

pub async fn manifest_module(
    state: &AppState,
    server_id: i64,
    info: &Value,
) -> ApiResult<Option<ModuleManifest>> {
    let deployment=sqlx::query("SELECT rev,bundle_sha256 FROM deployments WHERE server_id=$1 AND module='singbox' ORDER BY rev DESC LIMIT 1").bind(server_id).fetch_optional(&state.pool).await?;
    let Some(deployment) = deployment else {
        return Ok(None);
    };
    let revision = deployment.get("rev");
    match prepare_manifest(state, info, revision, deployment.get("bundle_sha256")).await {
        Ok(module) => {
            // A stale fetch must not clear another target's preparation error or
            // any error reported by the device itself.
            preparation_result(state, server_id, revision, None).await?;
            Ok(Some(module))
        }
        Err(error) => {
            let reason = match &error {
                ApiError::BadRequest(message) => message.clone(),
                ApiError::NotFound => {
                    "缺少此平台的已验签 sing-box 1.14.2 制品，请先导入可信发布制品".into()
                }
                ApiError::Conflict(_) => {
                    "sing-box 制品验签或完整性检查失败，请检查已导入的发布制品".into()
                }
                _ => "无法读取已签 sing-box 制品，请检查面板制品存储".into(),
            };
            let message = format!("{MANIFEST_ERROR_PREFIX}目标版本 {revision}：{reason}");
            // The panel knows this published target, not whether the device has
            // applied it. Preserve all device revision and health evidence.
            preparation_result(state, server_id, revision, Some(&message)).await?;
            Err(if matches!(error, ApiError::BadRequest(_)) {
                ApiError::BadRequest(message)
            } else {
                ApiError::Conflict(message)
            })
        }
    }
}

async fn prepare_manifest(
    state: &AppState,
    info: &Value,
    config_rev: i64,
    bundle_sha256: String,
) -> ApiResult<ModuleManifest> {
    let arch = match info["arch"].as_str() {
        Some("aarch64" | "arm64") => "arm64",
        Some("x86_64" | "amd64") => "amd64",
        _ => return Err(ApiError::BadRequest("设备架构未知".into())),
    };
    let runtime_libc = match info.get("runtime_libc") {
        Some(serde_json::Value::String(libc))
            if info["os"] == "linux" && matches!(libc.as_str(), "gnu" | "glibc" | "musl") =>
        {
            Some(libc.as_str())
        }
        Some(_) => {
            return Err(ApiError::BadRequest(
                "设备运行时 libc 未知或未受支持".into(),
            ));
        }
        None => info["libc"].as_str(),
    };
    let target = info["os"]
        .as_str()
        .and_then(|os| sinan_protocol::platform::artifact_target(os, runtime_libc, arch));
    if info["os"].is_string() && target.is_none() {
        return Err(ApiError::BadRequest("设备平台或 libc 未受支持".into()));
    }
    let mut targets = Vec::new();
    if let Some(target) = target {
        let gnu_host = info["os"] == "linux" && matches!(runtime_libc, Some("gnu" | "glibc"));
        let compiled_target = info["os"].as_str().and_then(|os| {
            sinan_protocol::platform::artifact_target(os, info["libc"].as_str(), arch)
        });
        let preserve_legacy =
            info["os"] == "linux" && compiled_target.as_ref().is_some_and(|old| old != &target);
        if preserve_legacy {
            // Preserve caches from either Linux ABI compatibility direction.
            targets.push(compiled_target.expect("checked compiled target"));
            targets.push(arch.into());
        }
        targets.push(target);
        if gnu_host && !preserve_legacy {
            targets.push(arch.into());
        }
    } else {
        targets.push(arch.into());
    }
    let mut artifact = None;
    for target in targets {
        match artifacts::descriptor(state, "sing-box", "1.14.2", &target).await {
            Ok(found) => {
                artifact = Some(found);
                break;
            }
            Err(ApiError::NotFound) => continue,
            Err(error) => return Err(error),
        }
    }
    let artifact = artifact.ok_or(ApiError::NotFound)?;
    Ok(ModuleManifest {
        kernel_version: "1.14.2".into(),
        artifact,
        config_rev: config_rev as u64,
        bundle_url: format!(
            "{}/api/agent/v1/bundles/{config_rev}",
            state.config.public_url
        ),
        bundle_sha256,
        stats_listen: "127.0.0.1:18085".into(),
    })
}

pub async fn bundle(state: &AppState, server_id: i64, rev: i64) -> ApiResult<String> {
    sqlx::query_scalar(
        "SELECT bundle FROM deployments WHERE server_id=$1 AND module='singbox' AND rev=$2",
    )
    .bind(server_id)
    .bind(rev)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(ApiError::NotFound)
}
