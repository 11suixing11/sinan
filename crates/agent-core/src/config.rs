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
    pub agent_root: PathBuf,
    pub status_socket: PathBuf,
    pub operation_timeout_secs: u64,
    pub public_ips: Vec<String>,
    pub settings: sinan_protocol::AgentSettings,
}

impl Default for Config {
    fn default() -> Self {
        let mut config = Self {
            panel_url: String::new(),
            identity_dir: "/etc/sinan/identity".into(),
            state_db: "/var/lib/sinan/core/state.db".into(),
            runtime_root: "/var/lib/sinan/plugins".into(),
            install_root: "/opt/sinan/plugins".into(),
            agent_root: "/opt/sinan/core".into(),
            status_socket: "/run/sinan/agent.sock".into(),
            operation_timeout_secs: 30,
            public_ips: Vec::new(),
            settings: sinan_protocol::AgentSettings::default(),
        };
        if cfg!(windows) {
            let root = std::env::var_os("ProgramData")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("C:\\ProgramData"))
                .join("Sinan");
            config.identity_dir = root.join("identity");
            config.state_db = root.join("state/core/state.db");
            config.runtime_root = root.join("state/plugins");
            config.install_root = root.join("plugins");
            config.agent_root = root.join("core");
            config.status_socket = root.join("state/core/status.json");
            config.operation_timeout_secs = 120;
        } else if cfg!(any(target_os = "macos", target_os = "freebsd")) {
            config.status_socket = "/var/run/sinan/agent.sock".into();
        }
        config
    }
}

pub fn default_path() -> PathBuf {
    if cfg!(windows) {
        Config::default()
            .identity_dir
            .parent()
            .expect("default identity parent")
            .join("agent.toml")
    } else {
        "/etc/sinan/agent.toml".into()
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
            &self.agent_root,
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
    let host = url.host_str().unwrap_or_default();
    let literal = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(host);
    let loopback = host == "localhost"
        || literal
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if url.scheme() == "http" && !loopback {
        bail!(
            "panel URL must use HTTPS; HTTP is restricted to literal loopback addresses or localhost"
        );
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_transport_requires_https_outside_literal_loopback() {
        for origin in [
            "https://panel.example.test",
            "https://192.0.2.1:8443",
            "https://[2001:db8::1]",
            "http://127.0.0.1:8080",
            "http://127.12.34.56",
            "http://[::1]:8080",
            "http://localhost:8080",
        ] {
            assert!(
                validate_panel_url(origin).is_ok(),
                "valid origin rejected: {origin}"
            );
        }
        for origin in [
            "http://panel.example.test",
            "http://localhost.example.test",
            "http://192.0.2.1",
            "http://10.0.0.1",
            "http://[2001:db8::1]",
            "http://[::]",
            "ftp://localhost",
            "https://user:password@panel.example.test",
            "https://panel.example.test/path",
            "https://panel.example.test?token=test",
            "https://panel.example.test#fragment",
        ] {
            assert!(
                validate_panel_url(origin).is_err(),
                "invalid origin accepted: {origin}"
            );
        }
    }

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
