pub use sinan_adapter_sdk::{Privileged, ServiceManager};
#[path = "execution.rs"]
mod execution;
#[path = "services.rs"]
mod services;
pub use services::{ServiceBackend, SystemServiceManager};
#[path = "archive.rs"]
mod archive;
#[path = "deploy.rs"]
pub mod deploy;

use crate::artifacts::safe_component;
use anyhow::{Context, Result, ensure};
use sinan_adapter_sdk::{BoxFuture, CommandOutput, Execution, JobStatus, ServiceJob};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::process::Command;
use uuid::Uuid;

const COMMAND_TIMEOUT: Duration = Duration::from_secs(30);
#[derive(Default)]
pub struct SystemOps;

pub fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

impl SystemOps {
    pub async fn protect(&self, path: &Path, mode: u32, group: Option<&str>) -> Result<()> {
        let mut grants = String::new();
        if let Some(group) = group {
            ensure!(safe_component(group), "invalid runtime account");
            let rights = if mode & 0o020 != 0 {
                "Modify"
            } else {
                "ReadAndExecute"
            };
            grants.push_str(&format!("$sid=(New-Object System.Security.Principal.NTAccount({})).Translate([System.Security.Principal.SecurityIdentifier]); Add-Rule $sid '{rights}';",ps_quote(group)));
        }
        if mode & 0o004 != 0 {
            grants.push_str("Add-Rule ([System.Security.Principal.SecurityIdentifier]'S-1-5-32-545') 'ReadAndExecute';");
        }
        let script = format!(
            r#"$ErrorActionPreference='Stop'; $p={}; $item=Get-Item -LiteralPath $p -Force; if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {{ throw 'Reparse points are not allowed' }}; $acl=if($item.PSIsContainer) {{ New-Object System.Security.AccessControl.DirectorySecurity }} else {{ New-Object System.Security.AccessControl.FileSecurity }}; $acl.SetAccessRuleProtection($true,$false); $inherit=if($item.PSIsContainer) {{ [Security.AccessControl.InheritanceFlags]'ContainerInherit,ObjectInherit' }} else {{ [Security.AccessControl.InheritanceFlags]::None }}; function Add-Rule($sid,$rights) {{ $rule=New-Object Security.AccessControl.FileSystemAccessRule($sid,$rights,$inherit,[Security.AccessControl.PropagationFlags]::None,[Security.AccessControl.AccessControlType]::Allow); $acl.AddAccessRule($rule) }}; $owner=[Security.Principal.WindowsIdentity]::GetCurrent().User; $acl.SetOwner($owner); Add-Rule $owner 'FullControl'; Add-Rule ([Security.Principal.SecurityIdentifier]'S-1-5-18') 'FullControl'; Add-Rule ([Security.Principal.SecurityIdentifier]'S-1-5-32-544') 'FullControl'; {grants} Set-Acl -LiteralPath $p -AclObject $acl"#,
            ps_quote(&path.to_string_lossy())
        );
        let output = self
            .execute_bounded(
                Path::new("powershell.exe"),
                &[
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    script,
                ],
                90,
                1024 * 1024,
            )
            .await
            .with_context(|| format!("protect Windows path {}", path.display()))?;
        ensure!(
            !output.timed_out,
            "protecting Windows path {} exceeded 90 seconds",
            path.display()
        );
        ensure!(
            output.output.success,
            "cannot protect Windows state directory: {}",
            output.output.stderr
        );
        Ok(())
    }
}

pub fn check_private(path: &Path) -> Result<()> {
    let script = format!(
        r#"$ErrorActionPreference='Stop'; $allowed=@([Security.Principal.WindowsIdentity]::GetCurrent().User.Value,'S-1-5-18','S-1-5-32-544'); $acl=Get-Acl -LiteralPath {}; $allowed+=([Security.Principal.NTAccount]$acl.Owner).Translate([Security.Principal.SecurityIdentifier]).Value; foreach($rule in $acl.GetAccessRules($true,$true,[Security.Principal.SecurityIdentifier])) {{ if($rule.AccessControlType -eq 'Allow' -and $allowed -notcontains $rule.IdentityReference.Value) {{ throw 'State grants access to another account' }} }}"#,
        ps_quote(&path.to_string_lossy())
    );
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()?;
    ensure!(
        output.status.success(),
        "device identity or status file is not private"
    );
    Ok(())
}

impl Privileged for SystemOps {
    fn spawn_managed<'a>(
        &'a self,
        program: &'a Path,
        args: &'a [String],
    ) -> BoxFuture<'a, Box<dyn sinan_adapter_sdk::ManagedProcess>> {
        Box::pin(async move { execution::spawn(program, args) })
    }
    fn execute<'a>(
        &'a self,
        program: &'a Path,
        args: &'a [String],
    ) -> BoxFuture<'a, CommandOutput> {
        Box::pin(async move {
            let output = execution::execute(program, args, 30, 1024 * 1024).await?;
            ensure!(!output.timed_out, "command exceeded 30 seconds");
            Ok(output.output)
        })
    }
    fn execute_bounded<'a>(
        &'a self,
        program: &'a Path,
        args: &'a [String],
        seconds: u32,
        maximum: usize,
    ) -> BoxFuture<'a, Execution> {
        Box::pin(execution::execute(program, args, seconds, maximum))
    }
    fn create_dir<'a>(
        &'a self,
        path: &'a Path,
        mode: u32,
        group: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            let mut missing = Vec::new();
            let mut current = path;
            while !current.exists() {
                missing.push(current.to_owned());
                current = current
                    .parent()
                    .context("directory has no existing ancestor")?;
            }
            for directory in missing.into_iter().rev() {
                fs::create_dir(&directory)?;
                self.protect(&directory, 0o755, None).await?;
            }
            self.protect(path, mode, group).await
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
            let parent = path.parent().context("file has no parent")?;
            if !parent.exists() {
                self.create_dir(parent, 0o700, None).await?;
            }
            let temporary = parent.join(format!(".write-{}", Uuid::new_v4()));
            let result = async {
                let mut file = fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&temporary)?;
                file.write_all(bytes)?;
                file.sync_all()?;
                drop(file);
                self.protect(&temporary, mode, group).await?;
                fs::rename(&temporary, path)?;
                Ok::<_, anyhow::Error>(())
            }
            .await;
            if result.is_err() {
                let _ = fs::remove_file(temporary);
            }
            result
        })
    }
    fn atomic_symlink<'a>(&'a self, link: &'a Path, target: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if link.exists() {
                read_reference(link)?;
            }
            self.write_file(
                link,
                &serde_json::to_vec(
                    &serde_json::json!({"sinan_directory_reference":true,"target":target}),
                )?,
                0o644,
                None,
            )
            .await
        })
    }
    fn remove_symlink<'a>(&'a self, link: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            if link.exists() {
                read_reference(link)?;
                fs::remove_file(link)?;
            }
            Ok(())
        })
    }
    fn install_archive<'a>(
        &'a self,
        path: &'a Path,
        directory: &'a Path,
        binary_name: &'a str,
    ) -> BoxFuture<'a, ()> {
        self.install_archive_files(path, directory, binary_name, &[])
    }
    fn install_archive_files<'a>(
        &'a self,
        path: &'a Path,
        directory: &'a Path,
        binary_name: &'a str,
        extras: &'a [String],
    ) -> BoxFuture<'a, ()> {
        Box::pin(async move {
            self.create_dir(
                directory.parent().context("artifact has no parent")?,
                0o755,
                None,
            )
            .await?;
            let (source, destination, name, files) = (
                path.to_owned(),
                directory.to_owned(),
                binary_name.to_owned(),
                extras.to_vec(),
            );
            tokio::task::spawn_blocking(move || {
                archive::install(&source, &destination, &name, &files)
            })
            .await??;
            for name in std::iter::once(binary_name).chain(extras.iter().map(String::as_str)) {
                self.protect(&directory.join(name), 0o755, None).await?;
            }
            self.protect(directory, 0o755, None).await
        })
    }
}

pub fn read_reference(path: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && metadata.len() <= 16 * 1024,
        "invalid directory reference"
    );
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    ensure!(
        value["sinan_directory_reference"] == true,
        "refusing to replace an ordinary file"
    );
    Ok(PathBuf::from(
        value["target"]
            .as_str()
            .context("reference has no target")?,
    ))
}

impl SystemServiceManager {
    fn start_diagnostic_job<'a>(&'a self, _job: &'a ServiceJob) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("diagnostic jobs require Linux") })
    }
    fn diagnostic_job_status<'a>(&'a self, _unit: &'a str) -> BoxFuture<'a, JobStatus> {
        Box::pin(async { anyhow::bail!("diagnostic jobs require Linux") })
    }
}
