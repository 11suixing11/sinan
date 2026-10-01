mod network;

use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use anyhow::{Context, Result, ensure};
use axum::{Json, extract::State, http::HeaderMap};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sinan_protocol::{
    PROTOCOL_MAX, PROTOCOL_MIN,
    release::{
        MAX_CHECKSUMS_BYTES, MAX_METADATA_BYTES, MAX_SIGNATURE_BYTES, ReleaseProof, TrustedKeys,
        VerifiedArtifact, VerifiedRelease, canonical_path, verify_release,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    future::Future,
    io::Read,
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

const MAX_ARTIFACT: usize = 512 * 1024 * 1024;
const MAX_INSTALLER: usize = 256 * 1024;
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;
type Identity = (String, String, String);

struct StoredRelease {
    directory: PathBuf,
    proof: ReleaseProof,
    verified: VerifiedRelease,
}

pub fn valid_tag(tag: &str) -> bool {
    tag.strip_prefix("agent-v").is_some_and(|version| {
        !version.is_empty()
            && version.len() <= 96
            && version
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-'))
    })
}

fn keys(state: &AppState) -> ApiResult<&TrustedKeys> {
    state.release_keys.as_deref().ok_or_else(|| {
        ApiError::Conflict("面板构建时未配置发布公钥，请使用可信公钥重新构建".into())
    })
}

fn invalid(error: impl std::fmt::Display) -> ApiError {
    tracing::warn!(error = %error, "signed release validation failed");
    ApiError::Conflict("发布签名、制品内容或路径校验失败，当前制品保持不变".into())
}

// Validate every existing ancestor before reading or creating managed storage.
fn checked(condition: bool, message: &str) -> ApiResult<()> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message))
    }
}

async fn ordinary_directory(path: &Path, create: bool) -> Result<bool> {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut prefix = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => continue,
            Component::ParentDir => {
                anyhow::bail!("release storage cannot contain parent components")
            }
            _ => prefix.push(component.as_os_str()),
        }
        let metadata = match tokio::fs::symlink_metadata(&prefix).await {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if !create {
                    return Ok(false);
                }
                match tokio::fs::create_dir(&prefix).await {
                    Ok(()) => (),
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => (),
                    Err(error) => return Err(error.into()),
                }
                tokio::fs::symlink_metadata(&prefix).await?
            }
            Err(error) => return Err(error.into()),
        };
        ensure!(
            metadata.is_dir() && !metadata.is_symlink(),
            "release directory or ancestor must not be a symlink"
        );
    }
    Ok(true)
}

async fn ordinary_bytes(path: &Path, maximum: usize) -> Result<Vec<u8>> {
    ensure!(
        ordinary_directory(path.parent().context("release file parent")?, false).await?,
        "release file parent is missing"
    );
    let metadata = tokio::fs::symlink_metadata(path).await?;
    ensure!(
        metadata.is_file() && !metadata.is_symlink() && metadata.len() <= maximum as u64,
        "release file must be a bounded ordinary file"
    );
    let bytes = tokio::fs::read(path).await?;
    ensure!(bytes.len() <= maximum, "release file grew beyond its limit");
    Ok(bytes)
}

pub async fn proof_at(directory: &Path) -> Result<ReleaseProof> {
    ensure!(
        ordinary_directory(directory, false).await?,
        "release directory is missing"
    );
    Ok(ReleaseProof {
        metadata_json: String::from_utf8(
            ordinary_bytes(&directory.join("release.json"), MAX_METADATA_BYTES).await?,
        )?,
        checksums: String::from_utf8(
            ordinary_bytes(&directory.join("SHA256SUMS"), MAX_CHECKSUMS_BYTES).await?,
        )?,
        signature: String::from_utf8(
            ordinary_bytes(&directory.join("SHA256SUMS.minisig"), MAX_SIGNATURE_BYTES).await?,
        )?,
    })
}

pub fn verify_payload(artifact: &VerifiedArtifact, bytes: &[u8]) -> Result<()> {
    artifact.verify_archive(bytes)?;
    let entry = artifact.metadata();
    if entry.format == "raw" {
        artifact.verify_binary(bytes)?;
        return Ok(());
    }
    let decoder = flate2::read::MultiGzDecoder::new(bytes).take(MAX_UNPACKED + 1);
    let mut archive = tar::Archive::new(decoder);
    let expected: BTreeSet<_> = std::iter::once(entry.binary_name.as_str())
        .chain(entry.auxiliary_files.keys().map(String::as_str))
        .collect();
    let mut seen = BTreeSet::new();
    for item in archive.entries()?.raw(true) {
        let mut item = item?;
        let name = std::str::from_utf8(item.path_bytes().as_ref())?.to_owned();
        ensure!(
            item.header().entry_type().is_file()
                && expected.contains(name.as_str())
                && seen.insert(name.clone()),
            "archive has an unexpected or duplicate file"
        );
        let size = if name == entry.binary_name {
            entry.binary_size
        } else {
            entry.auxiliary_files[&name].size
        };
        ensure!(item.header().size()? == size, "archive file size differs");
        let mut content = Vec::new();
        item.by_ref().take(size + 1).read_to_end(&mut content)?;
        if name == entry.binary_name {
            artifact.verify_binary(&content)?;
        } else {
            ensure!(
                content.len() as u64 == size
                    && format!("{:x}", Sha256::digest(&content))
                        == entry.auxiliary_files[&name].sha256,
                "auxiliary file digest or size differs"
            );
        }
    }
    ensure!(
        seen.iter().map(String::as_str).collect::<BTreeSet<_>>() == expected,
        "archive is missing a signed file"
    );
    let mut decoder = archive.into_inner();
    let mut tail = [0u8; 8192];
    loop {
        let count = decoder.read(&mut tail)?;
        if count == 0 {
            break;
        }
        ensure!(
            tail[..count].iter().all(|value| *value == 0),
            "archive contains trailing data"
        );
    }
    ensure!(decoder.limit() > 0, "archive exceeds unpacked size limit");
    Ok(())
}

async fn released(state: &AppState) -> ApiResult<Vec<StoredRelease>> {
    let trusted = keys(state)?;
    let root = state.config.data_dir.join("artifacts/releases");
    if !ordinary_directory(&root, false).await.map_err(invalid)? {
        return Ok(Vec::new());
    }
    let mut directories = tokio::fs::read_dir(&root)
        .await
        .map_err(anyhow::Error::from)?;
    let mut releases = Vec::new();
    while let Some(directory) = directories
        .next_entry()
        .await
        .map_err(anyhow::Error::from)?
    {
        let tag = directory.file_name().to_string_lossy().into_owned();
        if !valid_tag(&tag) {
            continue;
        }
        if releases.len() >= 128 {
            return Err(ApiError::Conflict("发布数量超过扫描上限".into()));
        }
        let proof = proof_at(&directory.path()).await.map_err(invalid)?;
        let verified = verify_release(&proof, trusted).map_err(invalid)?;
        checked(
            verified.metadata().tag == tag,
            "release directory and signed tag differ",
        )?;
        releases.push(StoredRelease {
            directory: directory.path(),
            proof,
            verified,
        });
    }
    releases.sort_by(|a, b| a.directory.cmp(&b.directory));
    Ok(releases)
}

fn same_identity(old: &VerifiedArtifact, new: &VerifiedArtifact) -> bool {
    let a = old.metadata();
    let b = new.metadata();
    old.sha256() == new.sha256()
        && a.binary_sha256 == b.binary_sha256
        && a.archive_size == b.archive_size
        && a.binary_size == b.binary_size
        && a.binary_name == b.binary_name
        && a.format == b.format
        && a.auxiliary_files == b.auxiliary_files
}

fn inventory(releases: &[StoredRelease]) -> Result<BTreeMap<Identity, (usize, VerifiedArtifact)>> {
    let mut result = BTreeMap::new();
    for (index, release) in releases.iter().enumerate() {
        for entry in &release.verified.metadata().artifacts {
            let key = (
                entry.name.clone(),
                entry.version.clone(),
                entry.arch.clone(),
            );
            let artifact = release
                .verified
                .artifact(&entry.name, &entry.version, &entry.arch)?;
            if let Some((_, old)) = result.get(&key) {
                ensure!(
                    same_identity(old, &artifact),
                    "an immutable artifact identity has different bytes"
                );
            } else {
                result.insert(key, (index, artifact));
            }
        }
    }
    Ok(result)
}

async fn stored_bytes(release: &StoredRelease, artifact: &VerifiedArtifact) -> Result<Vec<u8>> {
    let bytes = ordinary_bytes(&release.directory.join(artifact.path()), MAX_ARTIFACT).await?;
    verify_payload(artifact, &bytes)?;
    Ok(bytes)
}

pub async fn artifact(
    state: &AppState,
    name: &str,
    version: &str,
    arch: &str,
) -> ApiResult<(Vec<u8>, String, ReleaseProof)> {
    canonical_path(name, version, arch).map_err(|_| ApiError::NotFound)?;
    let releases = released(state).await?;
    let inventory = inventory(&releases).map_err(invalid)?;
    let key = (name.to_owned(), version.to_owned(), arch.to_owned());
    let (index, artifact) = inventory.get(&key).ok_or(ApiError::NotFound)?;
    let release = &releases[*index];
    let bytes = stored_bytes(release, artifact).await.map_err(invalid)?;
    Ok((bytes, artifact.sha256().to_owned(), release.proof.clone()))
}

pub async fn entries(state: &AppState) -> ApiResult<Vec<crate::artifacts::ArtifactEntry>> {
    // Scan and verify proofs once; each unique payload is validated once per listing.
    let releases = released(state).await?;
    let mut values = Vec::new();
    for (_, (index, artifact)) in inventory(&releases).map_err(invalid)? {
        stored_bytes(&releases[index], &artifact)
            .await
            .map_err(invalid)?;
        let entry = artifact.metadata();
        values.push(crate::artifacts::ArtifactEntry {
            name: entry.name.clone(),
            version: entry.version.clone(),
            arch: entry.arch.clone(),
            sha256: artifact.sha256().to_owned(),
            bytes: entry.archive_size,
        });
    }
    Ok(values)
}

pub async fn select_agent(state: &AppState, version: Option<&str>) -> ApiResult<(String, String)> {
    let mut candidates = Vec::new();
    for release in released(state).await? {
        let metadata = release.verified.metadata();
        if metadata.protocol_min > PROTOCOL_MAX || metadata.protocol_max < PROTOCOL_MIN {
            continue;
        }
        for entry in &metadata.artifacts {
            if entry.name != "agent" || version.is_some_and(|v| v != entry.version) {
                continue;
            }
            let numbers: Option<Vec<u64>> =
                entry.version.split('.').map(|v| v.parse().ok()).collect();
            if let Some(numbers) = numbers.filter(|v| v.len() == 3) {
                candidates.push((numbers, entry.version.clone(), metadata.tag.clone()));
            }
        }
    }
    candidates.sort();
    candidates
        .pop()
        .map(|(_, version, tag)| (version, tag))
        .ok_or_else(|| ApiError::Conflict("请先导入协议兼容且已签名的 Agent Release".into()))
}

/// Returns only protocol-compatible, signed updates for the requested ABI.
pub async fn newer_agent(
    state: &AppState,
    targets: &[String],
    current: (u64, u64, u64),
) -> ApiResult<Option<sinan_protocol::AgentRelease>> {
    let releases = released(state).await?;
    let inventory = inventory(&releases).map_err(invalid)?;
    let mut candidates = Vec::new();
    for ((name, version, target), (index, artifact)) in inventory {
        let metadata = releases[index].verified.metadata();
        if name != "agent"
            || metadata.protocol_min > PROTOCOL_MAX
            || metadata.protocol_max < PROTOCOL_MIN
        {
            continue;
        }
        let Some(priority) = targets.iter().position(|value| value == &target) else {
            continue;
        };
        let Some(key) = sinan_protocol::release_version(&version).filter(|key| *key > current)
        else {
            continue;
        };
        candidates.push((
            key,
            std::cmp::Reverse(priority),
            version,
            target,
            index,
            artifact,
        ));
    }
    candidates.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
    let Some((_, _, version, _, index, artifact)) = candidates.pop() else {
        return Ok(None);
    };
    let release = &releases[index];
    Ok(Some(sinan_protocol::AgentRelease {
        version: version.clone(),
        download_mirror: String::new(),
        artifact: sinan_protocol::Artifact {
            url: format!(
                "https://github.com/{}/releases/download/{}/{}",
                release.verified.metadata().source_repo,
                release.verified.metadata().tag,
                artifact.metadata().asset_name
            ),
            sha256: artifact.sha256().into(),
            proof: Some(release.proof.clone()),
        },
    }))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportRequest {
    pub tag: String,
}

async fn synced_write(path: &Path, bytes: &[u8]) -> Result<()> {
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await?;
    file.write_all(bytes).await?;
    file.sync_all().await?;
    Ok(())
}

async fn sync_directory(path: &Path) -> Result<()> {
    ensure!(
        ordinary_directory(path, false).await?,
        "release directory is missing"
    );
    tokio::fs::File::open(path).await?.sync_all().await?;
    Ok(())
}

/// Validates and atomically stores a release. Callers provide bytes, never arbitrary URLs.
/// Only the HTTP handler binds the asset loader to the fixed GitHub repository.
pub async fn import_bundle<F, Fut>(
    state: &AppState,
    tag: &str,
    proof: ReleaseProof,
    asset_bytes: F,
) -> ApiResult<usize>
where
    F: FnMut(String, usize) -> Fut,
    Fut: Future<Output = Result<Vec<u8>>>,
{
    if !valid_tag(tag) {
        return Err(ApiError::BadRequest(
            "请输入 agent-v 开头的规范发布标签".into(),
        ));
    }
    let _permit = state
        .release_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Busy)?;
    store_bundle(state, tag, proof, asset_bytes).await
}

async fn store_bundle<F, Fut>(
    state: &AppState,
    tag: &str,
    proof: ReleaseProof,
    mut asset_bytes: F,
) -> ApiResult<usize>
where
    F: FnMut(String, usize) -> Fut,
    Fut: Future<Output = Result<Vec<u8>>>,
{
    let verified = verify_release(&proof, keys(state)?).map_err(invalid)?;
    checked(verified.metadata().tag == tag, "signed tag differs")?;
    let releases = released(state).await?;
    let existing = inventory(&releases).map_err(invalid)?;
    for entry in &verified.metadata().artifacts {
        if let Some((_, old)) = existing.get(&(
            entry.name.clone(),
            entry.version.clone(),
            entry.arch.clone(),
        )) {
            let artifact = verified
                .artifact(&entry.name, &entry.version, &entry.arch)
                .map_err(invalid)?;
            checked(
                same_identity(old, &artifact),
                "an immutable artifact identity already has different bytes",
            )?;
        }
    }
    if let Some(old) = releases
        .iter()
        .find(|release| release.verified.metadata().tag == tag)
    {
        checked(
            old.proof.checksums == proof.checksums,
            "immutable release already differs",
        )?;
        let installer = ordinary_bytes(&old.directory.join("install.sh"), MAX_INSTALLER)
            .await
            .map_err(invalid)?;
        checked(
            verified.checksum("install.sh")
                == Some(format!("{:x}", Sha256::digest(&installer)).as_str()),
            "installer digest differs",
        )?;
        for entry in &verified.metadata().artifacts {
            let artifact = verified
                .artifact(&entry.name, &entry.version, &entry.arch)
                .map_err(invalid)?;
            stored_bytes(old, &artifact).await.map_err(invalid)?;
        }
        return Ok(verified.metadata().artifacts.len());
    }
    let root = state.config.data_dir.join("artifacts/releases");
    ordinary_directory(&root, true).await.map_err(invalid)?;
    let directory = root.join(format!(".staging-{}", Uuid::new_v4()));
    tokio::fs::create_dir(&directory)
        .await
        .map_err(anyhow::Error::from)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(anyhow::Error::from)?;
    }
    let result = async {
        synced_write(
            &directory.join("release.json"),
            proof.metadata_json.as_bytes(),
        )
        .await?;
        synced_write(&directory.join("SHA256SUMS"), proof.checksums.as_bytes()).await?;
        synced_write(
            &directory.join("SHA256SUMS.minisig"),
            proof.signature.as_bytes(),
        )
        .await?;
        let installer = asset_bytes("install.sh".into(), MAX_INSTALLER).await?;
        ensure!(
            installer.len() <= MAX_INSTALLER
                && verified.checksum("install.sh")
                    == Some(format!("{:x}", Sha256::digest(&installer)).as_str()),
            "installer digest differs"
        );
        synced_write(&directory.join("install.sh"), &installer).await?;
        let mut parents = BTreeSet::new();
        for entry in &verified.metadata().artifacts {
            let artifact = verified.artifact(&entry.name, &entry.version, &entry.arch)?;
            let bytes =
                asset_bytes(entry.asset_name.clone(), entry.archive_size.try_into()?).await?;
            verify_payload(&artifact, &bytes)?;
            let output = directory.join(artifact.path());
            let parent = output.parent().context("artifact parent")?;
            ordinary_directory(parent, true).await?;
            synced_write(&output, &bytes).await?;
            parents.insert(parent.to_owned());
            parents.insert(parent.parent().context("component parent")?.to_owned());
        }
        for parent in parents.iter().rev() {
            sync_directory(parent).await?;
        }
        sync_directory(&directory).await?;
        ensure!(
            ordinary_directory(&root, false).await?,
            "release root disappeared"
        );
        let output = root.join(tag);
        match tokio::fs::symlink_metadata(&output).await {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            _ => anyhow::bail!("release destination appeared during import"),
        }
        tokio::fs::rename(&directory, &output).await?;
        sync_directory(&root).await?;
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if result.is_err() {
        let _ = tokio::fs::remove_dir_all(&directory).await;
    }
    result.map_err(invalid)?;
    Ok(verified.metadata().artifacts.len())
}

pub async fn import(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ImportRequest>,
) -> ApiResult<Json<Value>> {
    auth::require_admin(&state, &headers).await?;
    if !valid_tag(&request.tag) {
        return Err(ApiError::BadRequest(
            "请输入 agent-v 开头的规范发布标签".into(),
        ));
    }
    let _permit = state
        .release_permits
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Busy)?;
    let proof = network::release_proof(&request.tag, keys(&state)?)
        .await
        .map_err(invalid)?;
    let tag = request.tag.clone();
    let count = store_bundle(&state, &request.tag, proof, move |asset, maximum| {
        let tag = tag.clone();
        async move { network::asset(&tag, &asset, maximum).await }
    })
    .await?;
    Ok(Json(
        json!({"tag": request.tag, "artifacts": count, "signature_verified": true}),
    ))
}
