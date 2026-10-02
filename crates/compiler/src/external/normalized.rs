use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum ExternalProtocol {
    Shadowsocks,
    Vmess,
    Trojan,
    Vless,
    Hysteria2,
    Tuic,
    Anytls,
    Socks,
    Http,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Common {
    pub server: String,
    pub server_port: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub network: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<Tls>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transport: Option<Transport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiplex: Option<Multiplex>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udp_over_tcp: Option<UdpOverTcp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_timeout: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp_fast_open: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp_multi_path: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udp_fragment: Option<bool>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Tls {
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disable_sni: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insecure: Option<bool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alpn: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cipher_suites: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub curve_preferences: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub certificate: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub certificate_public_key_sha256: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub client_certificate: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub client_key: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utls: Option<Utls>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reality: Option<Reality>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ech: Option<Ech>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Utls {
    pub enabled: bool,
    pub fingerprint: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Reality {
    pub enabled: bool,
    pub public_key: String,
    pub short_id: String,
}
#[derive(Clone, Serialize, Deserialize)]
pub struct Ech {
    pub enabled: bool,
    pub config: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Transport {
    Ws {
        path: String,
        headers: BTreeMap<String, Vec<String>>,
        max_early_data: u32,
        early_data_header_name: String,
    },
    Http {
        host: Vec<String>,
        path: String,
        method: String,
        headers: BTreeMap<String, Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        idle_timeout: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ping_timeout: Option<String>,
    },
    Grpc {
        service_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        idle_timeout: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ping_timeout: Option<String>,
        permit_without_stream: bool,
    },
    Httpupgrade {
        host: String,
        path: String,
        headers: BTreeMap<String, Vec<String>>,
    },
    Quic {},
}

impl Transport {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Ws { .. } => "ws",
            Self::Http { .. } => "http",
            Self::Grpc { .. } => "grpc",
            Self::Httpupgrade { .. } => "httpupgrade",
            Self::Quic {} => "quic",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Multiplex {
    pub enabled: bool,
    pub protocol: String,
    pub max_connections: u16,
    pub min_streams: u16,
    pub max_streams: u16,
    pub padding: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UdpOverTcp {
    pub enabled: bool,
    pub version: u8,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum HysteriaObfs {
    Salamander {
        password: String,
    },
    Gecko {
        password: String,
        min_packet_size: u16,
        max_packet_size: u16,
    },
}

// Only validated, normalized fields reach this type; it never contains a source URL or arbitrary config.
#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum NormalizedOutbound {
    Shadowsocks {
        #[serde(flatten)]
        common: Common,
        method: String,
        password: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        plugin: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        plugin_opts: Option<String>,
    },
    Vmess {
        #[serde(flatten)]
        common: Common,
        uuid: String,
        security: String,
        alter_id: u16,
        global_padding: bool,
        authenticated_length: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        packet_encoding: Option<String>,
    },
    Trojan {
        #[serde(flatten)]
        common: Common,
        password: String,
    },
    Vless {
        #[serde(flatten)]
        common: Common,
        uuid: String,
        flow: String,
        packet_encoding: String,
    },
    Hysteria2 {
        #[serde(flatten)]
        common: Common,
        password: String,
        server_ports: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hop_interval: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        hop_interval_max: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        up_mbps: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        down_mbps: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        obfs: Option<HysteriaObfs>,
        bbr_profile: String,
        disable_chrome_parrot: bool,
    },
    Tuic {
        #[serde(flatten)]
        common: Common,
        uuid: String,
        password: String,
        congestion_control: String,
        udp_relay_mode: String,
        udp_over_stream: bool,
        zero_rtt_handshake: bool,
        heartbeat: String,
    },
    Anytls {
        #[serde(flatten)]
        common: Common,
        password: String,
        idle_session_check_interval: String,
        idle_session_timeout: String,
        min_idle_session: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        client_metadata: Option<String>,
    },
    Socks {
        #[serde(flatten)]
        common: Common,
        version: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        username: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
    },
    Http {
        #[serde(flatten)]
        common: Common,
        #[serde(skip_serializing_if = "Option::is_none")]
        username: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        password: Option<String>,
        path: String,
        headers: BTreeMap<String, Vec<String>>,
    },
}

impl NormalizedOutbound {
    pub fn common(&self) -> &Common {
        match self {
            Self::Shadowsocks { common, .. }
            | Self::Vmess { common, .. }
            | Self::Trojan { common, .. }
            | Self::Vless { common, .. }
            | Self::Hysteria2 { common, .. }
            | Self::Tuic { common, .. }
            | Self::Anytls { common, .. }
            | Self::Socks { common, .. }
            | Self::Http { common, .. } => common,
        }
    }

    pub fn protocol(&self) -> ExternalProtocol {
        match self {
            Self::Shadowsocks { .. } => ExternalProtocol::Shadowsocks,
            Self::Vmess { .. } => ExternalProtocol::Vmess,
            Self::Trojan { .. } => ExternalProtocol::Trojan,
            Self::Vless { .. } => ExternalProtocol::Vless,
            Self::Hysteria2 { .. } => ExternalProtocol::Hysteria2,
            Self::Tuic { .. } => ExternalProtocol::Tuic,
            Self::Anytls { .. } => ExternalProtocol::Anytls,
            Self::Socks { .. } => ExternalProtocol::Socks,
            Self::Http { .. } => ExternalProtocol::Http,
        }
    }

    pub fn tcp(&self) -> bool {
        self.common().network.is_empty()
            || !self.common().network.iter().all(|network| network == "udp")
    }
    pub fn udp(&self) -> bool {
        !matches!(self, Self::Http { .. })
            && !matches!(self, Self::Socks {version,..} if version != "5")
            && (self.common().network.is_empty()
                || self.common().network.iter().any(|network| network == "udp"))
    }
}
