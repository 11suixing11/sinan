use crate::{INPUT_LIMIT, OUTPUT_LIMIT, Options, Report, Snapshot, model::now_millis};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    fs,
    io::{AsyncReadExt, AsyncWriteExt},
    time::{Instant, timeout, timeout_at},
};

const IO_LIMIT: Duration = Duration::from_secs(2);

pub struct Journal {
    workspace: PathBuf,
    targets_file: String,
    digest: String,
    snapshot: Snapshot,
    revisions: std::collections::BTreeMap<String, u64>,
}

#[derive(Serialize)]
struct Section<'a> {
    name: &'a str,
    text: String,
    complete: bool,
    revision: u64,
    collected_at: u64,
}

async fn regular_private(path: &Path, directory: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path).await?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "symlinks are not permitted"
    );
    ensure!(
        if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
        },
        "workspace/input must be an ordinary directory/file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        ensure!(
            metadata.permissions().mode() & 0o077 == 0,
            "workspace/input must be private"
        );
        if !directory {
            ensure!(metadata.nlink() == 1, "input hard links are not permitted");
        }
    }
    #[cfg(not(unix))]
    anyhow::bail!("private workspace verification requires Unix");
    Ok(())
}

impl Journal {
    pub async fn open(options: &Options) -> Result<Self> {
        ensure!(options.valid(), "invalid probe options");
        timeout(IO_LIMIT, async {
            regular_private(&options.workspace, true).await?;
            ensure!(
                fs::canonicalize(&options.workspace).await? == options.workspace,
                "workspace ancestors must not be symlinks"
            );
            for file in ["result.json", "sections"] {
                match fs::symlink_metadata(options.workspace.join(file)).await {
                    Err(error) if error.kind() == ErrorKind::NotFound => (),
                    _ => anyhow::bail!("workspace already contains diagnostic output"),
                }
            }
            let input = options.workspace.join(&options.targets_file);
            regular_private(&input, false).await?;
            let file = fs::File::open(&input).await?;
            ensure!(
                file.metadata().await?.len() <= INPUT_LIMIT as u64,
                "target snapshot exceeds 16 KiB"
            );
            let mut bytes = Vec::new();
            file.take(INPUT_LIMIT as u64 + 1)
                .read_to_end(&mut bytes)
                .await?;
            ensure!(bytes.len() <= INPUT_LIMIT, "target snapshot exceeds 16 KiB");
            ensure!(
                format!("{:x}", Sha256::digest(&bytes)) == options.target_digest,
                "target snapshot digest mismatch"
            );
            let snapshot: Snapshot =
                serde_json::from_slice(&bytes).context("invalid target snapshot")?;
            ensure!(snapshot.valid(), "invalid target snapshot");
            let directory = options.workspace.join("sections");
            tokio::task::spawn_blocking(move || -> Result<()> {
                let mut builder = std::fs::DirBuilder::new();
                #[cfg(unix)]
                {
                    use std::os::unix::fs::DirBuilderExt;
                    builder.mode(0o700);
                }
                builder.create(directory)?;
                Ok(())
            })
            .await??;
            Ok(Self {
                workspace: options.workspace.clone(),
                targets_file: options.targets_file.clone(),
                digest: options.target_digest.clone(),
                snapshot,
                revisions: Default::default(),
            })
        })
        .await
        .context("workspace/input inspection timed out")?
    }

    pub(crate) fn snapshot(&self, options: &Options) -> Result<Snapshot> {
        ensure!(
            options.valid()
                && options.workspace == self.workspace
                && options.targets_file == self.targets_file
                && options.target_digest == self.digest,
            "workspace/input identity changed"
        );
        Ok(self.snapshot.clone())
    }

    pub(crate) async fn update(
        &mut self,
        report: &Report,
        index: Option<usize>,
        deadline: Instant,
    ) -> Result<()> {
        if let Some(index) = index {
            let target = &report.targets[index];
            let name = format!(
                "tcp_target_{}",
                target.target.id.replace('-', "").to_ascii_lowercase()
            );
            self.section(&name, target, target.complete, deadline)
                .await?;
        } else {
            self.section("tcp_scope", report, true, deadline).await?;
        }
        self.section("tcp_summary", report, report.complete, deadline)
            .await?;
        let bytes = serde_json::to_vec_pretty(report)?;
        ensure!(bytes.len() <= OUTPUT_LIMIT, "report exceeds 64 KiB");
        self.atomic(&self.workspace.join("result.json"), &bytes, deadline)
            .await
    }

    async fn section(
        &mut self,
        name: &str,
        value: &impl Serialize,
        complete: bool,
        deadline: Instant,
    ) -> Result<()> {
        let revision = self.revisions.entry(name.into()).or_default();
        *revision += 1;
        let section = Section {
            name,
            text: serde_json::to_string_pretty(value)?,
            complete,
            revision: *revision,
            collected_at: now_millis()? / 1000,
        };
        let bytes = serde_json::to_vec(&section)?;
        ensure!(bytes.len() <= OUTPUT_LIMIT, "section exceeds 64 KiB");
        self.atomic(
            &self.workspace.join("sections").join(format!("{name}.json")),
            &bytes,
            deadline,
        )
        .await
    }

    async fn atomic(&self, target: &Path, bytes: &[u8], deadline: Instant) -> Result<()> {
        let filename = target
            .file_name()
            .and_then(|name| name.to_str())
            .context("invalid report file name")?;
        let pending = target.with_file_name(format!(".{filename}.pending"));
        let operation = async {
            let mut options = fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options.open(&pending).await?;
            file.write_all(bytes).await?;
            file.sync_all().await?;
            drop(file);
            fs::rename(&pending, target).await?;
            Ok::<_, anyhow::Error>(())
        };
        timeout_at(deadline.min(Instant::now() + IO_LIMIT), operation)
            .await
            .context("report publication timed out")?
    }
}
