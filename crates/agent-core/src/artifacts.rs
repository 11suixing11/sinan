use crate::config::validate_panel_url;
use anyhow::{Context, Result, ensure};
use futures_util::StreamExt;
use reqwest::{Client, Url, redirect::Policy};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sinan_adapter_sdk::{Descriptor, Privileged};
use sinan_protocol::{Artifact, Bundle, DiagnosticJob, DiagnosticUpdate, Manifest};
use std::{
    path::{Component, Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncReadExt;
use uuid::Uuid;

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
}

#[derive(Serialize, Deserialize)]
struct InstalledArtifact {
    archive_sha256: String,
    binary_sha256: String,
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
    pub(crate) async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.panel.join(path)?;
        Ok(serde_json::from_slice(
            &self.download(url.as_str(), 1024 * 1024).await?,
        )?)
    }

    pub(crate) async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        value: &impl Serialize,
    ) -> Result<T> {
        let response = self
            .client
            .post(self.panel.join(path)?)
            .bearer_auth(&self.session_token)
            .json(value)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "Agent request returned HTTP {}",
            response.status()
        );
        Ok(serde_json::from_slice(
            &Self::response_bytes(response, 16 * 1024).await?,
        )?)
    }

    async fn response_bytes(response: reqwest::Response, maximum: usize) -> Result<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            ensure!(
                chunk.len() <= maximum.saturating_sub(bytes.len()),
                "Agent response exceeds size limit"
            );
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
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
        })
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

    pub async fn agent_settings(&self) -> Result<sinan_protocol::AgentSettings> {
        let url = self.panel.join("/api/agent/v1/settings")?;
        let settings: sinan_protocol::AgentSettings =
            serde_json::from_slice(&self.download(url.as_str(), 16 * 1024).await?)?;
        ensure!(settings.valid(), "panel provided invalid Agent settings");
        Ok(settings)
    }

    pub async fn telemetry(
        &self,
        batch: &sinan_protocol::TelemetryBatch,
    ) -> Result<sinan_protocol::TelemetryAck> {
        use std::io::Write;
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(&serde_json::to_vec(batch)?)?;
        let response = self
            .client
            .post(self.panel.join("/api/agent/v1/telemetry")?)
            .bearer_auth(&self.session_token)
            .header(reqwest::header::CONTENT_ENCODING, "gzip")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(encoder.finish()?)
            .send()
            .await?;
        ensure!(
            response.status().is_success(),
            "telemetry upload returned HTTP {}",
            response.status()
        );
        let body = Self::response_bytes(response, 16 * 1024).await?;
        ensure!(
            body.len() <= 16 * 1024,
            "telemetry acknowledgment exceeds size limit"
        );
        let ack: sinan_protocol::TelemetryAck = serde_json::from_slice(&body)?;
        ensure!(
            ack.ids.len() <= 64
                && ack
                    .ids
                    .iter()
                    .all(|id| batch.samples.iter().any(|sample| sample.id == *id)),
            "panel acknowledged samples outside the submitted batch"
        );
        Ok(ack)
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
        self.validate_url(&artifact.url)?;
        let expected = normalized_hash(&artifact.sha256)?;
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
                let marker = directory.join(MARKER);
                let metadata = tokio::fs::symlink_metadata(&marker)
                    .await
                    .context("existing artifact has no cache metadata; refusing to overwrite")?;
                ensure!(
                    metadata.is_file() && metadata.len() <= 4096,
                    "invalid artifact cache metadata"
                );
                let cached: InstalledArtifact =
                    serde_json::from_slice(&tokio::fs::read(marker).await?)?;
                ensure!(
                    cached.archive_sha256 == expected,
                    "existing artifact version has a different SHA256; refusing to overwrite"
                );
                ensure!(
                    file_digest(&binary).await? == cached.binary_sha256,
                    "cached artifact binary was modified"
                );
                return Ok(binary);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let bytes = self.download(&artifact.url, MAX_DOWNLOAD).await?;
        ensure!(digest(&bytes) == expected, "artifact SHA256 mismatch");
        ops.create_dir(&plugin, 0o755, None).await?;
        let archive = plugin.join(format!(".download-{}.tar.gz", Uuid::new_v4()));
        ops.write_file(&archive, &bytes, 0o600, None).await?;
        let installed = ops
            .install_archive(&archive, &directory, &descriptor.binary_name)
            .await;
        let _ = ops
            .execute(
                Path::new("rm"),
                &[
                    "-f".into(),
                    "--".into(),
                    archive.to_string_lossy().into_owned(),
                ],
            )
            .await;
        installed?;
        let marker = InstalledArtifact {
            archive_sha256: expected,
            binary_sha256: file_digest(&binary).await?,
        };
        ops.write_file(
            &directory.join(MARKER),
            &serde_json::to_vec(&marker)?,
            0o644,
            None,
        )
        .await?;
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

async fn file_digest(path: &Path) -> Result<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut length = 0_u64;
    loop {
        let count = file.read(&mut buffer).await?;
        if count == 0 {
            break;
        }
        length += count as u64;
        ensure!(
            length <= 256 * 1024 * 1024,
            "cached artifact exceeds size limit"
        );
        hash.update(&buffer[..count]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
