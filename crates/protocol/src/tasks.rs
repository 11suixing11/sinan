use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteCommand {
    pub id: Uuid,
    pub command: String,
    pub timeout_secs: u32,
    pub expires_at: i64,
}

impl RemoteCommand {
    pub fn valid(&self) -> bool {
        !self.command.trim().is_empty()
            && self.command.len() <= 16_384
            && !self.command.contains('\0')
            && (1..=600).contains(&self.timeout_secs)
            && self.expires_at > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Succeeded,
    Failed,
    Expired,
    Interrupted,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub id: Uuid,
    pub status: CommandStatus,
    pub finished_at: i64,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeKind {
    Tcp,
    Icmp,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeAddressFamily {
    #[default]
    Any,
    Ipv4,
    Ipv6,
}

impl ProbeAddressFamily {
    pub fn allows(self, address: std::net::IpAddr) -> bool {
        match self {
            Self::Any => true,
            Self::Ipv4 => address.is_ipv4(),
            Self::Ipv6 => address.is_ipv6(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeAuthorizationKind {
    Owned,
    Consent,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeIdentity {
    pub kind: ProbeKind,
    pub target: String,
    pub port: Option<u16>,
    pub address_family: ProbeAddressFamily,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeAuthorization {
    pub kind: ProbeAuthorizationKind,
    pub source: String,
    pub scope: String,
    pub enabled: bool,
    pub expires_at: Option<i64>,
    pub identity: ProbeIdentity,
}

impl ProbeAuthorization {
    fn valid(&self) -> bool {
        valid_note(&self.source, 256, false)
            && valid_note(&self.scope, 512, false)
            && self.expires_at.is_none_or(|expires| (1..=253_402_300_799).contains(&expires))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeMonitor {
    pub region: String,
    pub address_family: ProbeAddressFamily,
    pub authorization: Option<ProbeAuthorization>,
}

fn valid_note(value: &str, limit: usize, allow_empty: bool) -> bool {
    value.len() <= limit
        && (allow_empty || !value.trim().is_empty())
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeSpec {
    pub id: Uuid,
    pub name: String,
    pub kind: ProbeKind,
    pub target: String,
    pub port: Option<u16>,
    pub interval_secs: u32,
    pub carrier: String,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<ProbeMonitor>,
    // Presentation only; execution always validates the actual bound authorization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_authorized: Option<bool>,
}

impl ProbeSpec {
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name.len() <= 128
            && self.carrier.len() <= 64
            && !self.name.chars().any(char::is_control)
            && !self.carrier.chars().any(char::is_control)
            && !self.target.is_empty()
            && self.target.len() <= 253
            && !self.target.starts_with('-')
            && self
                .target
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".:-".contains(&b))
            && (10..=3600).contains(&self.interval_secs)
            && self.monitor.as_ref().is_none_or(|monitor| {
                valid_note(&monitor.region, 64, true)
                    && monitor.authorization.as_ref().is_none_or(|authorization| {
                        authorization.valid() && authorization.identity == self.identity()
                    })
            })
            && match self.kind {
                ProbeKind::Tcp => self.port.is_some_and(|p| p > 0),
                ProbeKind::Icmp => self.port.is_none(),
            }
    }

    pub fn address_family(&self) -> ProbeAddressFamily {
        self.monitor.as_ref().map(|monitor| monitor.address_family).unwrap_or_default()
    }

    pub fn identity(&self) -> ProbeIdentity {
        ProbeIdentity {
            kind: self.kind,
            target: self.target.clone(),
            port: self.port,
            address_family: self.address_family(),
        }
    }

    pub fn authorized_at(&self, timestamp: i64) -> bool {
        self.valid()
            && self.monitor.as_ref().and_then(|monitor| monitor.authorization.as_ref())
                .is_some_and(|authorization| authorization.enabled
                    && authorization.expires_at.is_none_or(|expires| timestamp < expires))
    }

    pub fn runnable_at(&self, timestamp: i64) -> bool {
        self.enabled && self.authorized_at(timestamp)
    }

    pub fn same_measurement_identity(&self, previous: &Self) -> bool {
        self.identity() == previous.identity()
            && self.carrier == previous.carrier
            && self.monitor.as_ref().map(|monitor| monitor.region.as_str()).unwrap_or("")
                == previous.monitor.as_ref().map(|monitor| monitor.region.as_str()).unwrap_or("")
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ProbeResult {
    pub id: Uuid,
    pub probe_id: Uuid,
    pub sampled_at: i64,
    pub latency_ms: Option<f64>,
    pub loss_percent: f64,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_family: Option<ProbeAddressFamily>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeBatch {
    pub results: Vec<ProbeResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskAck {
    pub ids: Vec<Uuid>,
}
