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
    pub public_ips: Vec<String>,
    pub settings: sinan_protocol::AgentSettings,
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
            public_ips: Vec::new(),
            settings: sinan_protocol::AgentSettings::default(),
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
        if !self.settings.valid() {
            bail!(
                "telemetry intervals must be within 1..=60 seconds and upload cannot precede sampling"
            );
        }
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
        if self.public_ips.len() > 32 {
            bail!("public_ips may contain at most 32 addresses");
        }
        for value in &self.public_ips {
            let address: std::net::IpAddr = value.parse().context("invalid public IP address")?;
            let address = match address {
                std::net::IpAddr::V6(address) => address
                    .to_ipv4_mapped()
                    .map(std::net::IpAddr::V4)
                    .unwrap_or(std::net::IpAddr::V6(address)),
                address => address,
            };
            if address.is_unspecified() || address.is_multicast() || address.is_loopback() {
                bail!("public_ips cannot contain unspecified, multicast, or loopback addresses");
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_ip_override_is_optional_and_validated() {
        let mut config: Config = toml::from_str("panel_url = 'http://127.0.0.1:8080'").unwrap();
        assert!(config.public_ips.is_empty());
        config.public_ips = vec!["192.0.2.1".into(), "2001:db8::1".into()];
        assert!(config.validate().is_ok());
        for value in [
            "example.com",
            "192.0.2.1:443",
            "::",
            "127.0.0.1",
            "::ffff:127.0.0.1",
            "224.0.0.1",
        ] {
            config.public_ips = vec![value.into()];
            assert!(
                config.validate().is_err(),
                "invalid override accepted: {value}"
            );
        }
    }
}
