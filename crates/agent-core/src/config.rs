use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub panel_url: String,
    pub identity_dir: PathBuf,
    pub state_db: PathBuf,
    pub runtime_root: PathBuf,
    pub install_root: PathBuf,
    pub status_socket: PathBuf,
    pub operation_timeout_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            panel_url: String::new(),
            identity_dir: "/etc/sinan/identity".into(),
            state_db: "/var/lib/sinan/core/state.db".into(),
            runtime_root: "/var/lib/sinan/plugins".into(),
            install_root: "/opt/sinan/plugins".into(),
            status_socket: "/run/sinan/agent.sock".into(),
            operation_timeout_secs: 30,
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let config: Self = toml::from_str(
            &std::fs::read_to_string(path)
                .with_context(|| format!("read configuration {}", path.display()))?,
        )?;
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        validate_panel_url(&self.panel_url)?;
        for path in [
            &self.identity_dir,
            &self.state_db,
            &self.runtime_root,
            &self.install_root,
            &self.status_socket,
        ] {
            if !path.is_absolute() {
                bail!("configured paths must be absolute: {}", path.display());
            }
        }
        if self.operation_timeout_secs == 0 || self.operation_timeout_secs > 3600 {
            bail!("operation_timeout_secs must be between 1 and 3600");
        }
        Ok(())
    }
}

pub fn validate_panel_url(value: &str) -> anyhow::Result<reqwest::Url> {
    let url = reqwest::Url::parse(value).context("invalid panel URL")?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || value.chars().any(char::is_control)
    {
        bail!("panel URL must be an HTTP(S) origin without credentials, path, query, or fragment");
    }
    Ok(url)
}
