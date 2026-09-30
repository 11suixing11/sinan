use crate::config::validate_panel_url;
mod preflight;
#[cfg(test)]
mod tests;
mod verification;
use anyhow::{Context, Result, ensure};
use futures_util::StreamExt;
pub use preflight::verify_cache;
use reqwest::{Client, Url, redirect::Policy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sinan_adapter_sdk::{Descriptor, Privileged};
use sinan_protocol::release::{ReleaseError, ReleaseProof, TrustedKeys, VerifiedArtifact};
use sinan_protocol::{Artifact, Bundle, DiagnosticJob, DiagnosticUpdate, Manifest};
use std::{
    path::{Component, Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;
pub(crate) use verification::verify_expected;
pub use verification::{verify_installed_binary, verify_release_directory};

const MAX_DOWNLOAD: usize = 512 * 1024 * 1024;
const MAX_JSON: usize = 32 * 1024 * 1024;
const MARKER: &str = ".artifact.json";

pub(crate) fn safe_component(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !matches!(value, "." | "..")
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

#[derive(Clone)]
pub struct PanelClient {
    client: Client,
    panel: Url,
    session_token: String,
    keys: std::result::Result<TrustedKeys, ReleaseError>,
}

#[derive(Serialize, Deserialize)]
struct InstalledArtifact {
    #[serde(default)]
    proof: Option<ReleaseProof>,
}

fn normalized_hash(value: &str) -> Result<String> {
    ensure!(
        value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid SHA256 digest"
    );
    Ok(value.to_ascii_lowercase())
}

pub fn validate_bundle_files(bundle: &Bundle) -> Result<()> {
    ensure!(!bundle.files.is_empty(), "bundle cannot be empty");
    for name in bundle.files.keys() {
        let path = Path::new(name);
        ensure!(
            !name.is_empty()
                && name.len() <= 1024
                && !name.contains('\\')
                && !name.chars().any(char::is_control),
            "invalid bundle file name"
        );
        ensure!(
            !path.is_absolute()
                && name
                    .split('/')
                    .all(|part| !part.is_empty() && !matches!(part, "." | "..")),
            "bundle file must have a safe relative path"
        );
        ensure!(
            path.components()
                .all(|part| matches!(part, Component::Normal(_))),
            "invalid bundle file path"
        );
    }
    Ok(())
}

impl PanelClient {
    pub fn new(panel_url: &str, session_token: &str) -> Result<Self> {
        ensure!(
            !session_token.is_empty()
                && session_token.len() <= 512
                && !session_token.chars().any(char::is_control),
            "invalid device session token"
        );
        Ok(Self {
            client: Client::builder()
                .redirect(Policy::none())
                .connect_timeout(Duration::from_secs(20))
                .timeout(Duration::from_secs(120))
                .build()?,
            panel: validate_panel_url(panel_url)?,
            session_token: session_token.to_owned(),
            keys: TrustedKeys::compiled(),
        })
    }

    /// Explicit trusted roots for an embedding caller or deterministic tests.
    pub fn with_trusted_keys(mut self, keys: TrustedKeys) -> Self {
        self.keys = Ok(keys);
        self
    }

    pub(crate) fn keys(&self) -> Result<&TrustedKeys> {
        self.keys
            .as_ref()
            .map_err(|error| anyhow::anyhow!(error.to_string()))
    }

    pub(crate) fn verify_artifact(
        &self,
        artifact: &Artifact,
        version: &str,
        descriptor: &Descriptor,
    ) -> Result<VerifiedArtifact> {
        let url = self.validate_url(&artifact.url)?;
        let proof = artifact
            .proof
            .as_ref()
            .context("artifact has no signed release proof")?;
        let verified = verification::signed_artifact(proof, descriptor, version, self.keys()?)?;
        ensure!(
            url.path() == format!("/api/agent/v1/artifacts/{}", verified.path()),
            "artifact URL differs from signed identity"
        );
        ensure!(
            normalized_hash(&artifact.sha256)? == verified.sha256(),
            "artifact digest differs from signed release"
        );
        Ok(verified)
    }

    fn validate_url(&self, value: &str) -> Result<Url> {
        let url = Url::parse(value).context("invalid download URL")?;
        ensure!(
            url.origin() == self.panel.origin()
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "downloads must use the configured panel origin without credentials, query or fragment"
        );
        Ok(url)
    }

    async fn download(&self, value: &str, maximum: usize) -> Result<Vec<u8>> {
        let url = self.validate_url(value)?;
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.session_token)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "panel download returned HTTP {}",
            response.status()
        );
        if let Some(length) = response.content_length() {
            ensure!(length <= maximum as u64, "download exceeds size limit");
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            ensure!(
                chunk.len() <= maximum.saturating_sub(body.len()),
                "download exceeds size limit"
            );
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    pub async fn manifest(&self) -> Result<Manifest> {
        let url = self.panel.join("/api/agent/v1/manifest")?;
        Ok(serde_json::from_slice(
            &self.download(url.as_str(), MAX_JSON).await?,
        )?)
    }

    pub async fn diagnostic_jobs(&self) -> Result<Vec<DiagnosticJob>> {
        let url = self.panel.join("/api/agent/v1/diagnostics")?;
        let jobs: Vec<DiagnosticJob> =
            serde_json::from_slice(&self.download(url.as_str(), 1024 * 1024).await?)?;
        ensure!(jobs.len() <= 64, "too many pending diagnostic jobs");
        Ok(jobs)
    }

    pub async fn diagnostic_update(&self, update: &DiagnosticUpdate) -> Result<()> {
        let url = self
            .panel
            .join(&format!("/api/agent/v1/diagnostics/{}", update.id))?;
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.session_token)
            .json(update)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "diagnostic update returned HTTP {}",
            response.status()
        );
        Ok(())
    }

    pub async fn bundle(&self, url: &str, expected_sha256: &str) -> Result<Bundle> {
        let expected = normalized_hash(expected_sha256)?;
        let bytes = self.download(url, MAX_JSON).await?;
        ensure!(digest(&bytes) == expected, "bundle SHA256 mismatch");
        let bundle = serde_json::from_slice(&bytes)?;
        validate_bundle_files(&bundle)?;
        Ok(bundle)
    }

    pub async fn ensure_artifact(
        &self,
        artifact: &Artifact,
        version: &str,
        descriptor: &Descriptor,
        install_root: &Path,
        ops: &dyn Privileged,
    ) -> Result<PathBuf> {
        ensure!(
            install_root.is_absolute(),
            "artifact installation root must be absolute"
        );
        ensure!(
            safe_component(version)
                && safe_component(&descriptor.plugin_name)
                && safe_component(&descriptor.binary_name),
            "invalid artifact version or descriptor path"
        );
        let verified = self.verify_artifact(artifact, version, descriptor)?;
        ensure_ordinary_directory_if_present(install_root).await?;
        let plugin = install_root.join(&descriptor.plugin_name);
        let directory = plugin.join(version);
        let binary = directory.join(&descriptor.binary_name);
        ensure_ordinary_directory_if_present(&plugin).await?;
        match tokio::fs::symlink_metadata(&directory).await {
            Ok(metadata) => {
                ensure!(
                    metadata.is_dir(),
                    "cached artifact directory is not an ordinary directory"
                );
                ensure!(
                    tokio::fs::symlink_metadata(&binary).await?.is_file(),
                    "cached artifact is not an ordinary file"
                );
                let proof = verification::read_proof(&directory).await?;
                let cached =
                    verification::signed_artifact(&proof, descriptor, version, self.keys()?)?;
                ensure!(
                    cached.sha256() == verified.sha256(),
                    "existing artifact version has different signed contents"
                );
                verification::verify_file(&binary, &verified).await?;
                return Ok(binary);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let bytes = self.download(&artifact.url, MAX_DOWNLOAD).await?;
        verified.verify_archive(&bytes)?;
        ops.create_dir(&plugin, 0o755, None).await?;
        let archive = plugin.join(format!(".download-{}.tar.gz", Uuid::new_v4()));
        ops.write_file(&archive, &bytes, 0o600, None).await?;
        let staging = plugin.join(format!(".verified-{}", Uuid::new_v4()));
        let installed: Result<()> = async {
            ops.install_archive(&archive, &staging, &descriptor.binary_name)
                .await?;
            verification::verify_file(&staging.join(&descriptor.binary_name), &verified).await?;
            let proof = artifact
                .proof
                .as_ref()
                .context("artifact has no signed proof")?;
            for (name, bytes) in [
                ("release.json", proof.metadata_json.as_bytes()),
                ("SHA256SUMS", proof.checksums.as_bytes()),
                ("SHA256SUMS.minisig", proof.signature.as_bytes()),
            ] {
                ops.write_file(&staging.join(name), bytes, 0o644, None)
                    .await?;
            }
            let marker = InstalledArtifact {
                proof: Some(proof.clone()),
            };
            ops.write_file(
                &staging.join(MARKER),
                &serde_json::to_vec(&marker)?,
                0o644,
                None,
            )
            .await?;
            let moved = ops
                .execute(
                    Path::new("/bin/mv"),
                    &[
                        "--no-clobber".into(),
                        "--no-target-directory".into(),
                        "--".into(),
                        staging.to_string_lossy().into_owned(),
                        directory.to_string_lossy().into_owned(),
                    ],
                )
                .await?;
            ensure!(
                moved.success,
                "publishing verified artifact directory failed"
            );
            verify_expected(&binary, descriptor, version, self.keys()?).await?;
            let actual = verification::signed_artifact(
                &verification::read_proof(&directory).await?,
                descriptor,
                version,
                self.keys()?,
            )?;
            ensure!(
                actual.sha256() == verified.sha256(),
                "artifact publication raced with different contents"
            );
            Ok(())
        }
        .await;
        let _ = ops
            .execute(
                Path::new("rm"),
                &[
                    "-rf".into(),
                    "--".into(),
                    archive.to_string_lossy().into_owned(),
                    staging.to_string_lossy().into_owned(),
                ],
            )
            .await;
        installed?;
        Ok(binary)
    }
}

async fn ensure_ordinary_directory_if_present(path: &Path) -> Result<()> {
    match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => ensure!(
            metadata.is_dir(),
            "artifact plugin path is not an ordinary directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
