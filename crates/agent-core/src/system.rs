pub use sinan_adapter_sdk::{Privileged, ServiceManager};

use crate::artifacts::safe_component;
use anyhow::{ensure, Context, Result};
use flate2::read::MultiGzDecoder;
use sinan_adapter_sdk::{BoxFuture, CommandOutput};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{symlink, OpenOptionsExt, PermissionsExt},
    path::Path,
    sync::Arc,
    time::Duration,
};
use tokio::{process::Command, time::timeout};
use uuid::Uuid;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_UNPACKED: u64 = 256 * 1024 * 1024;

#[derive(Default)]
pub struct SystemOps;

fn ensure_directory(path: &Path) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Ok(());
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) => ensure!(
            metadata.is_dir()
                || (metadata.file_type().is_symlink() && fs::metadata(path)?.is_dir()),
            "directory path is not a directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                ensure_directory(parent)?;
            }
            match fs::create_dir(path) {
                Ok(()) => fs::set_permissions(path, fs::Permissions::from_mode(0o755))?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    ensure!(
                        fs::symlink_metadata(path)?.is_dir(),
                        "directory was replaced"
                    );
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn parent_directory(path: &Path) -> Result<&Path> {
    let parent = path.parent().context("path has no parent")?;
    ensure!(
        !parent.as_os_str().is_empty(),
        "path must have an explicit parent"
    );
    Ok(parent)
}

impl SystemOps {
    async fn assign_group(&self, path: &Path, group: Option<&str>) -> Result<()> {
        if let Some(group) = group.filter(|value| !value.is_empty()) {
            ensure!(safe_component(group), "invalid group name");
            let output = self
                .execute(
                    Path::new("chown"),
                    &[
                        "--".into(),
                        format!(":{group}"),
                        path.to_string_lossy().into_owned(),
                    ],
                )
                .await?;
            ensure!(
                output.success,
                "setting file group failed: {}",
                output.stderr
            );
        }
        Ok(())
    }
}

impl Privileged for SystemOps {
    fn execute<'a>(
        &'a self,
        program: &'a Path,
        args: &'a [String],
    ) -> BoxFuture<'a, CommandOutput> {
        Box::pin(async move {
            #[cfg(not(target_os = "linux"))]
            ensure!(
                program.file_name().is_none_or(|name| name != "systemctl"),
                "systemd operations require Linux"
            );
            let output = timeout(
                COMMAND_TIMEOUT,
                Command::new(program).args(args).kill_on_drop(true).output(),
            )
            .await
            .context("command exceeded 30 seconds")??;
            Ok(CommandOutput {
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            })
        })
    }

    fn create_dir<'a>(
        &'a self,
        path: &'a Path,
        mode: u32,
        group: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            ensure!(mode & !0o7777 == 0, "invalid permission mode");
            let owned = path.to_owned();
            tokio::task::spawn_blocking(move || ensure_directory(&owned)).await??;
            self.assign_group(path, group).await?;
            fs::set_permissions(path, fs::Permissions::from_mode(mode))?;
            sync_directory(path)?;
            Ok(())
        })
    }

    fn write_file<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
        mode: u32,
        group: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            ensure!(mode & !0o7777 == 0, "invalid permission mode");
            let parent = parent_directory(path)?;
            ensure_directory(parent)?;
            let temporary = parent.join(format!(".write-{}", Uuid::new_v4()));
            let result: Result<()> = async {
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .open(&temporary)?;
                file.write_all(bytes)?;
                file.sync_all()?;
                self.assign_group(&temporary, group).await?;
                file.set_permissions(fs::Permissions::from_mode(mode))?;
                file.sync_all()?;
                fs::rename(&temporary, path)?;
                sync_directory(parent)?;
                Ok(())
            }
            .await;
            if result.is_err() {
                let _ = fs::remove_file(&temporary);
            }
            result
        })
    }

    fn atomic_symlink<'a>(&'a self, link: &'a Path, target: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let parent = parent_directory(link)?;
            ensure_directory(parent)?;
            if let Ok(metadata) = fs::symlink_metadata(link) {
                ensure!(
                    metadata.file_type().is_symlink(),
                    "refusing to replace a non-symlink"
                );
            }
            let temporary = parent.join(format!(".link-{}", Uuid::new_v4()));
            symlink(target, &temporary)?;
            if let Err(error) = fs::rename(&temporary, link) {
                let _ = fs::remove_file(&temporary);
                return Err(error.into());
            }
            sync_directory(parent)
        })
    }

    fn remove_symlink<'a>(&'a self, link: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            match fs::symlink_metadata(link) {
                Ok(metadata) => {
                    ensure!(
                        metadata.file_type().is_symlink(),
                        "refusing to remove a non-symlink"
                    );
                    fs::remove_file(link)?;
                    sync_directory(parent_directory(link)?)?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            Ok(())
        })
    }

    fn install_archive<'a>(
        &'a self,
        archive: &'a Path,
        directory: &'a Path,
        binary_name: &'a str,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let archive = archive.to_owned();
            let directory = directory.to_owned();
            let binary_name = binary_name.to_owned();
            tokio::task::spawn_blocking(move || install_archive(&archive, &directory, &binary_name))
                .await?
        })
    }
}

fn install_archive(archive: &Path, directory: &Path, binary_name: &str) -> Result<()> {
    ensure!(safe_component(binary_name), "invalid binary name");
    ensure!(
        fs::symlink_metadata(directory).is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
        "artifact version already exists"
    );
    let parent = parent_directory(directory)?;
    ensure_directory(parent)?;
    let staging = parent.join(format!(".unpack-{}", Uuid::new_v4()));
    fs::create_dir(&staging)?;
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o700))?;
    let result = (|| -> Result<()> {
        let decoder = MultiGzDecoder::new(File::open(archive)?).take(MAX_UNPACKED + 1);
        let mut archive = tar::Archive::new(decoder);
        let mut found = false;
        for entry in archive.entries()?.raw(true) {
            let mut entry = entry?;
            ensure!(!found, "archive must contain exactly one file");
            ensure!(
                entry.header().entry_type().is_file(),
                "archive contains a non-ordinary file"
            );
            ensure!(
                entry.path_bytes().as_ref() == binary_name.as_bytes(),
                "archive file name does not match expected binary"
            );
            ensure!(
                entry.size() <= MAX_UNPACKED,
                "artifact exceeds unpacked size limit"
            );
            let output = staging.join(binary_name);
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o700)
                .open(output)?;
            let size = entry.size();
            ensure!(
                std::io::copy(&mut entry, &mut file)? == size,
                "archive file was truncated"
            );
            file.set_permissions(fs::Permissions::from_mode(0o755))?;
            file.sync_all()?;
            found = true;
        }
        ensure!(found, "archive does not contain the expected binary");
        let mut decoder = archive.into_inner();
        let mut tail = [0_u8; 8192];
        loop {
            let count = decoder.read(&mut tail)?;
            if count == 0 {
                break;
            }
            ensure!(
                tail[..count].iter().all(|byte| *byte == 0),
                "archive contains trailing data"
            );
        }
        ensure!(decoder.limit() > 0, "artifact exceeds unpacked size limit");
        fs::set_permissions(&staging, fs::Permissions::from_mode(0o755))?;
        sync_directory(&staging)?;
        ensure!(
            fs::symlink_metadata(directory)
                .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound),
            "artifact version appeared during installation"
        );
        fs::rename(&staging, directory)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(staging);
    }
    result
}

pub struct SystemServiceManager {
    privileged: Arc<dyn Privileged>,
}

impl SystemServiceManager {
    pub fn new(privileged: Arc<dyn Privileged>) -> Self {
        Self { privileged }
    }

    async fn call(&self, action: &str, unit: &str, quiet: bool) -> Result<CommandOutput> {
        ensure!(
            !unit.is_empty()
                && !unit.starts_with('-')
                && unit.len() <= 255
                && unit
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'@')),
            "invalid service unit"
        );
        let mut args = vec![action.to_owned()];
        if quiet {
            args.push("--quiet".into());
        }
        args.extend(["--".into(), unit.to_owned()]);
        timeout(
            COMMAND_TIMEOUT,
            self.privileged.execute(Path::new("systemctl"), &args),
        )
        .await
        .context("service operation timed out")?
    }

    async fn change(&self, action: &str, unit: &str) -> Result<()> {
        let output = self.call(action, unit, false).await?;
        ensure!(
            output.success,
            "service operation failed: {}",
            output.stderr
        );
        Ok(())
    }
}

impl ServiceManager for SystemServiceManager {
    fn reload<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(self.change("reload", unit))
    }
    fn restart<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(self.change("restart", unit))
    }
    fn stop<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(self.change("stop", unit))
    }
    fn is_active<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, bool> {
        Box::pin(async move { Ok(self.call("is-active", unit, true).await?.success) })
    }
}
