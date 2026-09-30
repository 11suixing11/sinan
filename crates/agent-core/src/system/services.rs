use super::{COMMAND_TIMEOUT, Privileged, ServiceManager};
use anyhow::{Context, Result, ensure};
use sinan_adapter_sdk::{BoxFuture, CommandOutput, JobStatus, ServiceJob};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::time::timeout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceBackend {
    Systemd,
    OpenRc,
    Launchd,
    FreeBsd,
    WindowsTask,
    Unmanaged,
}

impl ServiceBackend {
    pub fn detect() -> Result<Self> {
        if cfg!(target_os = "macos") {
            return Ok(Self::Launchd);
        }
        if cfg!(target_os = "freebsd") {
            return Ok(Self::FreeBsd);
        }
        if cfg!(windows) {
            return Ok(Self::WindowsTask);
        }
        Self::detect_at(
            Path::new("/run/systemd/system"),
            Path::new("/run/openrc/softlevel"),
        )
    }

    fn detect_at(systemd: &Path, openrc: &Path) -> Result<Self> {
        if systemd.is_dir() {
            Ok(Self::Systemd)
        } else if openrc.is_file() {
            Ok(Self::OpenRc)
        } else {
            anyhow::bail!("Agent 运行需要 Linux systemd 或 OpenRC")
        }
    }
}

pub struct SystemServiceManager {
    pub(super) privileged: Arc<dyn Privileged>,
    pub(super) backend: ServiceBackend,
    pub(super) job_root: PathBuf,
}

impl SystemServiceManager {
    pub fn new(privileged: Arc<dyn Privileged>, backend: ServiceBackend) -> Self {
        Self {
            privileged,
            backend,
            job_root: "/var/lib/sinan/core/service-jobs".into(),
        }
    }

    pub fn with_job_root(mut self, directory: PathBuf) -> Self {
        self.job_root = directory;
        self
    }

    async fn call(&self, action: &str, unit: &str, quiet: bool) -> Result<CommandOutput> {
        let service = unit.strip_suffix(".service").unwrap_or(unit);
        ensure!(
            !service.is_empty()
                && !matches!(service, "." | "..")
                && !service.starts_with('-')
                && unit.len() <= 255
                && unit
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'@')),
            "invalid service unit"
        );
        let (program, args) = match self.backend {
            ServiceBackend::Systemd => {
                let mut args = vec![action.to_owned()];
                if quiet {
                    args.push("--quiet".into());
                }
                args.extend(["--".into(), unit.to_owned()]);
                ("systemctl", args)
            }
            ServiceBackend::OpenRc => {
                let action = if action == "is-active" {
                    "status"
                } else {
                    action
                };
                (
                    "rc-service",
                    vec!["--".into(), service.into(), action.into()],
                )
            }
            ServiceBackend::FreeBsd => {
                let action = if action == "is-active" {
                    "onestatus"
                } else {
                    action
                };
                (
                    "service",
                    vec![service.replace(['@', '-', '.'], "_"), action.into()],
                )
            }
            ServiceBackend::Launchd => {
                let label = format!("system/org.sinan.{}", service.replace('@', "."));
                if matches!(action, "restart" | "stop") {
                    let loaded = self
                        .privileged
                        .execute(Path::new("launchctl"), &["print".into(), label.clone()])
                        .await?;
                    if action == "stop" && !loaded.success {
                        return Ok(CommandOutput {
                            success: true,
                            stdout: String::new(),
                            stderr: String::new(),
                        });
                    }
                    if action == "restart" && !loaded.success {
                        let path = format!(
                            "/Library/LaunchDaemons/org.sinan.{}.plist",
                            service.replace('@', ".")
                        );
                        let result = self
                            .privileged
                            .execute(
                                Path::new("launchctl"),
                                &["bootstrap".into(), "system".into(), path],
                            )
                            .await?;
                        ensure!(
                            result.success,
                            "cannot load runtime service: {}",
                            result.stderr
                        );
                    }
                }
                let args = match action {
                    "restart" => vec!["kickstart".into(), "-k".into(), label],
                    "reload" => vec!["kill".into(), "SIGHUP".into(), label],
                    "stop" => vec!["bootout".into(), label],
                    "is-active" => vec!["print".into(), label],
                    _ => anyhow::bail!("unsupported service action"),
                };
                ("launchctl", args)
            }
            ServiceBackend::WindowsTask => {
                let script = match action {
                    "restart" | "reload" => format!(
                        "$ErrorActionPreference='Stop'; Stop-ScheduledTask -TaskName '{service}'; Start-Sleep -Milliseconds 300; Start-ScheduledTask -TaskName '{service}'"
                    ),
                    "stop" => format!(
                        "$ErrorActionPreference='Stop'; Stop-ScheduledTask -TaskName '{service}'"
                    ),
                    "is-active" => format!(
                        "$ErrorActionPreference='Stop'; if ((Get-ScheduledTask -TaskName '{service}').State -ne 'Running') {{ exit 1 }}"
                    ),
                    _ => anyhow::bail!("unsupported service action"),
                };
                (
                    "powershell.exe",
                    vec![
                        "-NoProfile".into(),
                        "-NonInteractive".into(),
                        "-Command".into(),
                        script,
                    ],
                )
            }
            ServiceBackend::Unmanaged => {
                anyhow::bail!("monitor-only Agent has no runtime service manager")
            }
        };
        timeout(
            COMMAND_TIMEOUT,
            self.privileged.execute(Path::new(program), &args),
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
        Box::pin(async move {
            let output = self.call("is-active", unit, true).await?;
            Ok(output.success
                && (self.backend != ServiceBackend::Launchd
                    || output
                        .stdout
                        .lines()
                        .any(|line| line.trim() == "state = running")))
        })
    }
    fn start_job<'a>(&'a self, job: &'a ServiceJob) -> BoxFuture<'a, ()> {
        self.start_diagnostic_job(job)
    }
    fn job_status<'a>(&'a self, unit: &'a str) -> BoxFuture<'a, JobStatus> {
        self.diagnostic_job_status(unit)
    }
}

#[cfg(test)]
mod tests {
    use super::ServiceBackend;

    #[test]
    fn detects_active_init_and_prefers_systemd_over_openrc() {
        let root = std::env::temp_dir().join(format!("sinan-init-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let systemd = root.join("systemd");
        let openrc = root.join("softlevel");
        assert!(ServiceBackend::detect_at(&systemd, &openrc).is_err());
        std::fs::write(&openrc, "default\n").unwrap();
        assert_eq!(
            ServiceBackend::detect_at(&systemd, &openrc).unwrap(),
            ServiceBackend::OpenRc
        );
        std::fs::create_dir(&systemd).unwrap();
        assert_eq!(
            ServiceBackend::detect_at(&systemd, &openrc).unwrap(),
            ServiceBackend::Systemd
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
