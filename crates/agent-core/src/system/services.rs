use super::{COMMAND_TIMEOUT, Privileged, ServiceManager};
use anyhow::{Context, Result, ensure};
use sinan_adapter_sdk::{BoxFuture, CommandOutput};
use std::{path::Path, sync::Arc};
use tokio::time::timeout;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceBackend {
    Systemd,
    OpenRc,
}

impl ServiceBackend {
    pub fn detect() -> Result<Self> {
        ensure!(
            cfg!(target_os = "linux"),
            "service management requires Linux"
        );
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
    privileged: Arc<dyn Privileged>,
    backend: ServiceBackend,
}

impl SystemServiceManager {
    pub fn new(privileged: Arc<dyn Privileged>, backend: ServiceBackend) -> Self {
        Self {
            privileged,
            backend,
        }
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
        Box::pin(async move { Ok(self.call("is-active", unit, true).await?.success) })
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
