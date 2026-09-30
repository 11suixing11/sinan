#![forbid(unsafe_code)]

use anyhow::{bail, Context, Result};
use sinan_adapter_sdk::{
    BoxFuture, DiagnosticAdapter, DiagnosticDescriptor, DiagnosticOutput, DiagnosticSpec,
    Privileged, ServiceJob,
};
use std::{path::Path, time::Duration};
use tokio::{io::AsyncReadExt, time::timeout};

pub const VERSION: &str = "a92fca6c0067df29ddd03fdc2fee6f3000f64545";
pub const MAX_REPORT_BYTES: u64 = 256 * 1024;
const IO_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, Default)]
pub struct NodeQualityAdapter;

impl NodeQualityAdapter {
    pub fn new() -> Self {
        Self
    }
}

fn path_argument(path: &Path) -> Result<String> {
    let value = path.to_str().context("diagnostic path is not UTF-8")?;
    if !path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
        || value
            .chars()
            .any(|c| c.is_control() || c == '$' || c == '%')
    {
        bail!("diagnostic paths must be absolute without expansion or traversal");
    }
    Ok(value.into())
}

fn validate(spec: &DiagnosticSpec) -> Result<(String, String, String)> {
    if spec.version != VERSION {
        bail!("unsupported diagnostic version");
    }
    let id = spec.id.as_bytes();
    if id.len() != 36
        || id.iter().enumerate().any(|(i, c)| {
            if matches!(i, 8 | 13 | 18 | 23) {
                *c != b'-'
            } else {
                !c.is_ascii_hexdigit()
            }
        })
    {
        bail!("diagnostic id must be a UUID");
    }
    if !(1..=3600).contains(&spec.timeout_secs) {
        bail!("diagnostic timeout must be between 1 and 3600 seconds");
    }
    path_argument(&spec.binary_path)?;
    let workspace = path_argument(&spec.job_dir)?;
    if !workspace
        .bytes()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'/' | b'_' | b'.' | b'-'))
    {
        bail!("diagnostic workspace cannot contain whitespace or shell glob characters");
    }
    if spec.job_dir.parent().is_none() {
        bail!("diagnostic workspace must not be the filesystem root");
    }
    for key in spec.options.keys() {
        if !matches!(key.as_str(), "ip_version" | "network_mode") {
            bail!("unsupported diagnostic option");
        }
    }
    let ip_version = spec
        .options
        .get("ip_version")
        .map(String::as_str)
        .unwrap_or("both");
    let network_mode = spec
        .options
        .get("network_mode")
        .map(String::as_str)
        .unwrap_or("low");
    if !matches!(ip_version, "both" | "ipv4" | "ipv6") {
        bail!("invalid diagnostic IP version");
    }
    if !matches!(network_mode, "low" | "normal") {
        bail!("invalid diagnostic network mode");
    }
    Ok((workspace, ip_version.into(), network_mode.into()))
}

async fn read_bounded(path: &Path, limit: u64) -> Result<Option<String>> {
    let metadata = match tokio::fs::symlink_metadata(path).await {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !metadata.is_file() || metadata.len() > limit {
        bail!("diagnostic output is not a bounded ordinary file");
    }
    let file = tokio::fs::File::open(path).await?;
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).await?;
    if bytes.len() as u64 > limit {
        bail!("diagnostic output exceeded its size limit");
    }
    Ok(Some(String::from_utf8_lossy(&bytes).into_owned()))
}

fn valid_report_url(value: &str) -> bool {
    value
        .strip_prefix("https://nodequality.com/r/")
        .is_some_and(|token| {
            !token.is_empty()
                && token.len() <= 128
                && token
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
        })
}

impl DiagnosticAdapter for NodeQualityAdapter {
    fn describe(&self) -> DiagnosticDescriptor {
        DiagnosticDescriptor {
            plugin_name: "nodequality".into(),
            binary_name: "nodequality".into(),
        }
    }

    fn prepare<'a>(
        &'a self,
        spec: &'a DiagnosticSpec,
        privileged: &'a dyn Privileged,
    ) -> BoxFuture<'a, ServiceJob> {
        Box::pin(async move {
            let (workspace, ip_version, network_mode) = validate(spec)?;
            timeout(
                IO_TIMEOUT,
                privileged.create_dir(&spec.job_dir, 0o700, None),
            )
            .await
            .context("create diagnostic workspace timed out")??;
            let version_args = ["--version".into()];
            let output = timeout(
                IO_TIMEOUT,
                privileged.execute(&spec.binary_path, &version_args),
            )
            .await
            .context("diagnostic version verification timed out")??;
            if !output.success || output.stdout.trim() != format!("nodequality {VERSION}") {
                bail!("diagnostic artifact version verification failed");
            }
            Ok(ServiceJob {
                unit: format!("sinan-diagnostic-{}.service", spec.id),
                program: spec.binary_path.clone(),
                args: vec![
                    "--workspace".into(),
                    workspace,
                    "--ip-version".into(),
                    ip_version,
                    "--network-mode".into(),
                    network_mode,
                ],
                working_directory: spec.job_dir.clone(),
                timeout_secs: spec.timeout_secs,
            })
        })
    }

    fn collect<'a>(&'a self, spec: &'a DiagnosticSpec) -> BoxFuture<'a, Option<DiagnosticOutput>> {
        Box::pin(async move {
            validate(spec)?;
            timeout(IO_TIMEOUT, async {
                let result =
                    read_bounded(&spec.job_dir.join("result.txt"), MAX_REPORT_BYTES).await?;
                let text = match result {
                    Some(text) => text,
                    None => return Ok(None),
                };
                if text.trim().is_empty() {
                    return Ok(None);
                }
                let report_url =
                    match read_bounded(&spec.job_dir.join("report-url.txt"), 256).await? {
                        Some(url) if valid_report_url(url.trim()) => Some(url.trim().into()),
                        Some(_) => bail!("invalid diagnostic report URL"),
                        None => None,
                    };
                Ok(Some(DiagnosticOutput { text, report_url }))
            })
            .await
            .context("read diagnostic output timed out")?
        })
    }
}
