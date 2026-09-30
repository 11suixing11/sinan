use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sinan_protocol::Artifact;
use std::path::PathBuf;

pub const AGENT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Deserialize)]
pub struct TokenQuery {
    pub token: String,
}

#[derive(Serialize)]
pub struct ArtifactEntry {
    pub name: String,
    pub version: String,
    pub arch: String,
    pub sha256: String,
    pub bytes: u64,
}

fn safe_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value != "."
        && value != ".."
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

async fn artifact_path(
    state: &AppState,
    name: &str,
    version: &str,
    arch: &str,
) -> ApiResult<PathBuf> {
    if !matches!(name, "agent" | "sing-box" | "nodequality")
        || !safe_segment(version)
        || !matches!(arch, "amd64" | "arm64")
    {
        return Err(ApiError::NotFound);
    }
    let root = tokio::fs::canonicalize(state.config.data_dir.join("artifacts"))
        .await
        .map_err(|_| ApiError::NotFound)?;
    let path = tokio::fs::canonicalize(root.join(name).join(version).join(arch))
        .await
        .map_err(|_| ApiError::NotFound)?;
    if !path.starts_with(&root)
        || !tokio::fs::metadata(&path)
            .await
            .map_err(anyhow::Error::from)?
            .is_file()
    {
        return Err(ApiError::NotFound);
    }
    Ok(path)
}

async fn verified_bytes(
    state: &AppState,
    name: &str,
    version: &str,
    arch: &str,
) -> ApiResult<(Vec<u8>, String)> {
    let path = artifact_path(state, name, version, arch).await?;
    let root = tokio::fs::canonicalize(state.config.data_dir.join("artifacts"))
        .await
        .map_err(anyhow::Error::from)?;
    let sums = tokio::fs::canonicalize(
        state
            .config
            .data_dir
            .join("artifacts")
            .join(name)
            .join(version)
            .join("SHA256SUMS"),
    )
    .await
    .map_err(|_| ApiError::NotFound)?;
    if !sums.starts_with(&root) {
        return Err(ApiError::NotFound);
    }
    let sums = tokio::fs::read_to_string(sums)
        .await
        .map_err(anyhow::Error::from)?;
    let expected = sums
        .lines()
        .find_map(|line| {
            let mut fields = line.split_whitespace();
            let hash = fields.next()?;
            let file = fields.next()?.trim_start_matches('*');
            (file == arch
                && fields.next().is_none()
                && hash.len() == 64
                && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
        })
        .ok_or_else(|| ApiError::BadRequest("制品缺少有效 SHA256SUMS".into()))?;
    let bytes = tokio::fs::read(path).await.map_err(anyhow::Error::from)?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != expected {
        return Err(ApiError::Conflict(
            "制品校验失败，请重新上传制品及 SHA256SUMS".into(),
        ));
    }
    Ok((bytes, actual))
}

pub async fn descriptor(
    state: &AppState,
    name: &str,
    version: &str,
    arch: &str,
) -> ApiResult<Artifact> {
    let (_, sha256) = verified_bytes(state, name, version, arch).await?;
    Ok(Artifact {
        url: format!(
            "{}/api/agent/v1/artifacts/{name}/{version}/{arch}",
            state.config.public_url
        ),
        sha256,
    })
}

pub async fn download(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((name, version, arch)): Path<(String, String, String)>,
) -> ApiResult<Response> {
    auth::require_agent(&state, &headers).await?;
    bytes_response(&state, &name, &version, &arch).await
}

pub async fn bootstrap(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
    Path((version, arch)): Path<(String, String)>,
) -> ApiResult<Response> {
    crate::servers::validate_enrollment(&state.pool, &query.token).await?;
    bytes_response(&state, "agent", &version, &arch).await
}

async fn bytes_response(
    state: &AppState,
    name: &str,
    version: &str,
    arch: &str,
) -> ApiResult<Response> {
    let (bytes, _) = verified_bytes(state, name, version, arch).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/octet-stream"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bytes,
    )
        .into_response())
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ArtifactEntry>>> {
    auth::require_admin(&state, &headers).await?;
    let mut entries = Vec::new();
    for name in ["agent", "sing-box", "nodequality"] {
        let Ok(mut versions) =
            tokio::fs::read_dir(state.config.data_dir.join("artifacts").join(name)).await
        else {
            continue;
        };
        while let Some(version) = versions.next_entry().await.map_err(anyhow::Error::from)? {
            let version = version.file_name().to_string_lossy().into_owned();
            if !safe_segment(&version) {
                continue;
            }
            for arch in ["amd64", "arm64"] {
                if let Ok((bytes, sha256)) = verified_bytes(&state, name, &version, arch).await {
                    entries.push(ArtifactEntry {
                        name: name.into(),
                        version: version.clone(),
                        arch: arch.into(),
                        sha256,
                        bytes: bytes.len() as u64,
                    });
                }
            }
        }
    }
    entries.sort_by(|a, b| (&a.name, &a.version, &a.arch).cmp(&(&b.name, &b.version, &b.arch)));
    Ok(Json(entries))
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub async fn install_script(
    State(state): State<AppState>,
    Query(query): Query<TokenQuery>,
) -> ApiResult<Response> {
    crate::servers::validate_enrollment(&state.pool, &query.token).await?;
    let amd64 = descriptor(&state, "agent", AGENT_VERSION, "amd64")
        .await
        .ok()
        .map(|value| value.sha256)
        .unwrap_or_default();
    let arm64 = descriptor(&state, "agent", AGENT_VERSION, "arm64")
        .await
        .ok()
        .map(|value| value.sha256)
        .unwrap_or_default();
    if amd64.is_empty() && arm64.is_empty() {
        return Err(ApiError::NotFound);
    }
    let script = include_str!("../../../deploy/install.sh.tmpl")
        .replace("@@PANEL@@", &shell_quote(&state.config.public_url))
        .replace("@@TOKEN@@", &shell_quote(&query.token))
        .replace("@@VERSION@@", &shell_quote(AGENT_VERSION))
        .replace("@@AMD64_HASH@@", &shell_quote(&amd64))
        .replace("@@ARM64_HASH@@", &shell_quote(&arm64))
        .replace(
            "@@AGENT_UNIT@@",
            include_str!("../../../deploy/sinan-agent.service").trim_end(),
        )
        .replace(
            "@@RUNTIME_UNIT@@",
            include_str!("../../../plugins/sing-box/sinan-singbox@.service").trim_end(),
        )
        .replace(
            "@@AGENT_OPENRC@@",
            include_str!("../../../deploy/sinan-agent.openrc").trim_end(),
        )
        .replace(
            "@@RUNTIME_OPENRC@@",
            include_str!("../../../plugins/sing-box/sinan-singbox.openrc").trim_end(),
        );
    Ok((
        [
            (header::CONTENT_TYPE, "text/x-shellscript; charset=utf-8"),
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        script,
    )
        .into_response())
}
