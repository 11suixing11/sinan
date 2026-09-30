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
}

impl ProbeSpec {
    pub fn valid(&self) -> bool {
        !self.name.trim().is_empty()
            && self.name.len() <= 128
            && self.carrier.len() <= 64
            && !self.target.is_empty()
            && self.target.len() <= 253
            && !self.target.starts_with('-')
            && self
                .target
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".:-".contains(&b))
            && (10..=3600).contains(&self.interval_secs)
            && match self.kind {
                ProbeKind::Tcp => self.port.is_some_and(|p| p > 0),
                ProbeKind::Icmp => self.port.is_none(),
            }
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeBatch {
    pub results: Vec<ProbeResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskAck {
    pub ids: Vec<Uuid>,
}
