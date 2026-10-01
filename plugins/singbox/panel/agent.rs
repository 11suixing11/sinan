use crate::{
    AppState, artifacts,
    error::{ApiError, ApiResult},
};
use serde_json::Value;
use sinan_protocol::ModuleManifest;
use sqlx::Row;

pub async fn manifest_module(
    state: &AppState,
    server_id: i64,
    info: &Value,
) -> ApiResult<Option<ModuleManifest>> {
    let mut module = None;
    let deployment=sqlx::query("SELECT rev,bundle_sha256 FROM deployments WHERE server_id=$1 AND module='singbox' ORDER BY rev DESC LIMIT 1").bind(server_id).fetch_optional(&state.pool).await?;
    if let Some(deployment) = deployment {
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
        let config_rev: i64 = deployment.get("rev");
        module = Some(ModuleManifest {
            kernel_version: "1.14.2".into(),
            artifact,
            config_rev: config_rev as u64,
            bundle_url: format!(
                "{}/api/agent/v1/bundles/{config_rev}",
                state.config.public_url
            ),
            bundle_sha256: deployment.get("bundle_sha256"),
            stats_listen: "127.0.0.1:18085".into(),
        });
    }
    Ok(module)
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
