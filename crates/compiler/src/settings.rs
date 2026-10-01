use crate::{CompileError, Node, ProtocolConfig, invalid_node, valid_public_host};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, net::IpAddr};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NodeSettings {
    pub listen: String,
    pub public_port: Option<u16>,
    pub tcp_fast_open: bool,
    pub tls_alpn: Vec<String>,
    pub reality: RealitySettings,
    pub hysteria2: Hysteria2Settings,
    pub tuic: TuicSettings,
    pub anytls: AnyTlsSettings,
}

impl Default for NodeSettings {
    fn default() -> Self {
        Self {
            listen: "::".into(),
            public_port: None,
            tcp_fast_open: false,
            tls_alpn: vec![],
            reality: RealitySettings::default(),
            hysteria2: Hysteria2Settings::default(),
            tuic: TuicSettings::default(),
            anytls: AnyTlsSettings::default(),
        }
    }
}

impl NodeSettings {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RealitySettings {
    pub handshake_server: Option<String>,
    pub handshake_port: u16,
    pub fingerprint: Fingerprint,
}

impl Default for RealitySettings {
    fn default() -> Self {
        Self {
            handshake_server: None,
            handshake_port: 443,
            fingerprint: Fingerprint::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fingerprint {
    #[default]
    Chrome,
    Firefox,
    Safari,
    Edge,
    Ios,
    Android,
    Randomized,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Hysteria2Settings {
    pub up_mbps: Option<u32>,
    pub down_mbps: Option<u32>,
    pub ignore_client_bandwidth: bool,
    pub obfs_password: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CongestionControl {
    #[default]
    Cubic,
    NewReno,
    Bbr,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TuicSettings {
    pub congestion_control: CongestionControl,
    pub auth_timeout_seconds: Option<u16>,
    pub heartbeat_seconds: Option<u16>,
    pub zero_rtt_handshake: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AnyTlsSettings {
    pub idle_session_check_seconds: Option<u16>,
    pub idle_session_timeout_seconds: Option<u16>,
    pub min_idle_session: Option<u16>,
}

pub(crate) fn validate(node: &Node) -> Result<(), CompileError> {
    let value = &node.settings;
    let fail = |reason| invalid_node(node, reason);
    let address: IpAddr = value
        .listen
        .parse()
        .map_err(|_| fail("监听地址必须是 IPv4 或 IPv6 地址"))?;
    if address.is_multicast() || address == IpAddr::from([255, 255, 255, 255]) {
        return Err(fail("监听地址不能是组播或广播地址"));
    }
    if value.public_port == Some(0) {
        return Err(fail("公开端口必须为 1 至 65535"));
    }
    if value.tcp_fast_open && !node.protocol_config.uses_tcp() {
        return Err(fail("UDP 入站不能启用 TCP Fast Open"));
    }
    if !value.tls_alpn.is_empty() && node.protocol_config.tls().is_none() {
        return Err(fail("仅使用 TLS 证书的协议支持自定义 ALPN"));
    }
    let mut alpn = BTreeSet::new();
    if value.tls_alpn.len() > 8
        || value.tls_alpn.iter().any(|entry| {
            entry.is_empty()
                || entry.len() > 32
                || !entry.bytes().all(|b| b.is_ascii_graphic())
                || !alpn.insert(entry)
        })
    {
        return Err(fail(
            "ALPN 需要 1 至 8 个不重复的可见 ASCII 标识，每个最多 32 字节",
        ));
    }
    if matches!(node.protocol_config, ProtocolConfig::Naive { .. })
        && !value.tls_alpn.is_empty()
        && value.tls_alpn != ["h2"]
    {
        return Err(fail("Naive HTTP/2 的 ALPN 仅支持 h2"));
    }
    let reality = &value.reality;
    if !node.protocol_config.is_reality() && *reality != RealitySettings::default() {
        return Err(fail("当前协议不支持 Reality 参数"));
    }
    if reality.handshake_port == 0
        || reality
            .handshake_server
            .as_ref()
            .is_some_and(|host| !valid_public_host(host))
    {
        return Err(fail(
            "Reality 握手目标需为不含端口的域名或 IP，端口需为 1 至 65535",
        ));
    }
    let hy = &value.hysteria2;
    if !matches!(node.protocol_config, ProtocolConfig::Hysteria2 { .. })
        && *hy != Hysteria2Settings::default()
    {
        return Err(fail("当前协议不支持 Hysteria2 参数"));
    }
    if hy.up_mbps.is_some() != hy.down_mbps.is_some()
        || [hy.up_mbps, hy.down_mbps]
            .into_iter()
            .flatten()
            .any(|n| !(1..=1_000_000).contains(&n))
        || (hy.ignore_client_bandwidth && hy.up_mbps.is_some())
    {
        return Err(fail(
            "Hysteria2 上下行带宽需同时填写 1 至 1000000 Mbps，且不能同时强制 BBR",
        ));
    }
    if hy.obfs_password.as_ref().is_some_and(|secret| {
        !(8..=256).contains(&secret.len()) || secret.chars().any(char::is_control)
    }) {
        return Err(fail("混淆密码需为 8 至 256 字节且不含控制字符"));
    }
    if !matches!(node.protocol_config, ProtocolConfig::Tuic { .. })
        && value.tuic != TuicSettings::default()
    {
        return Err(fail("当前协议不支持 TUIC 参数"));
    }
    if !matches!(node.protocol_config, ProtocolConfig::Anytls { .. })
        && value.anytls != AnyTlsSettings::default()
    {
        return Err(fail("当前协议不支持 AnyTLS 参数"));
    }
    if [
        value.tuic.auth_timeout_seconds,
        value.tuic.heartbeat_seconds,
        value.anytls.idle_session_check_seconds,
        value.anytls.idle_session_timeout_seconds,
    ]
    .into_iter()
    .flatten()
    .any(|n| !(1..=3600).contains(&n))
        || value.anytls.min_idle_session.is_some_and(|n| n > 128)
    {
        return Err(fail(
            "超时和间隔需为 1 至 3600 秒，闲置会话数量需为 0 至 128",
        ));
    }
    Ok(())
}

fn seconds(target: &mut Value, field: &str, value: Option<u16>) {
    if let Some(value) = value {
        target[field] = json!(format!("{value}s"));
    }
}

pub(crate) fn apply(node: &Node, config: &mut Value, client: bool) {
    let settings = &node.settings;
    if !client {
        config["listen"] = json!(settings.listen);
        if settings.tcp_fast_open {
            config["tcp_fast_open"] = json!(true);
        }
    }
    // Naive negotiates HTTP/2 itself and rejects an explicit client ALPN.
    if !settings.tls_alpn.is_empty()
        && !(client && matches!(node.protocol_config, ProtocolConfig::Naive { .. }))
    {
        config["tls"]["alpn"] = json!(settings.tls_alpn);
    }
    match &node.protocol_config {
        ProtocolConfig::VlessReality => {
            if client {
                config["tls"]["utls"]["fingerprint"] = json!(settings.reality.fingerprint);
            } else {
                config["tls"]["reality"]["handshake"] = json!({
                    "server": crate::unbracket_host(settings.reality.handshake_server.as_deref().unwrap_or(&node.sni)),
                    "server_port": settings.reality.handshake_port
                });
            }
        }
        ProtocolConfig::Hysteria2 { .. } => {
            let hy = &settings.hysteria2;
            if let (Some(up), Some(down)) = (hy.up_mbps, hy.down_mbps) {
                config["up_mbps"] = json!(if client { down } else { up });
                config["down_mbps"] = json!(if client { up } else { down });
            }
            if !client && hy.ignore_client_bandwidth {
                config["ignore_client_bandwidth"] = json!(true);
            }
            if let Some(password) = &hy.obfs_password {
                config["obfs"] = json!({"type": "salamander", "password": password});
            }
        }
        ProtocolConfig::Tuic { .. } => {
            let tuic = &settings.tuic;
            if tuic.congestion_control != CongestionControl::default() {
                config["congestion_control"] = json!(tuic.congestion_control);
            }
            if tuic.zero_rtt_handshake {
                config["zero_rtt_handshake"] = json!(true);
            }
            seconds(config, "heartbeat", tuic.heartbeat_seconds);
            if !client {
                seconds(config, "auth_timeout", tuic.auth_timeout_seconds);
            }
        }
        ProtocolConfig::Anytls { .. } if client => {
            let value = &settings.anytls;
            seconds(
                config,
                "idle_session_check_interval",
                value.idle_session_check_seconds,
            );
            seconds(
                config,
                "idle_session_timeout",
                value.idle_session_timeout_seconds,
            );
            if let Some(count) = value.min_idle_session {
                config["min_idle_session"] = json!(count);
            }
        }
        _ => {}
    }
}
