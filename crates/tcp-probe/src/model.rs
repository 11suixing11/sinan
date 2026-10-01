use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    net::IpAddr,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IpVersion {
    #[serde(rename = "4")]
    V4,
    #[serde(rename = "6")]
    V6,
}

impl IpVersion {
    pub(crate) fn matches(self, address: IpAddr) -> bool {
        matches!(
            (self, address),
            (Self::V4, IpAddr::V4(_)) | (Self::V6, IpAddr::V6(_))
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub id: String,
    pub name: String,
    pub target: String,
    pub port: u16,
    pub carrier: String,
    #[serde(default)]
    pub region: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub schema: u32,
    pub targets: Vec<Target>,
}

fn identifier(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

pub(crate) fn unicast(address: IpAddr) -> bool {
    !address.is_unspecified()
        && !address.is_multicast()
        && !matches!(address, IpAddr::V4(address) if address.is_broadcast())
}

fn host(value: &str) -> bool {
    if let Ok(address) = value.parse::<IpAddr>() {
        return unicast(address);
    }
    if value.is_empty() || value.len() > 253 {
        return false;
    }
    value
        .strip_suffix('.')
        .unwrap_or(value)
        .split('.')
        .all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

impl Snapshot {
    pub fn valid(&self) -> bool {
        let mut identifiers = BTreeSet::new();
        self.schema == 1
            && (1..=8).contains(&self.targets.len())
            && self.targets.iter().all(|target| {
                identifier(&target.id)
                    && identifiers.insert(target.id.to_ascii_lowercase())
                    && text(&target.name, 128)
                    && host(&target.target)
                    && target.port > 0
                    && target.carrier.len() <= 64
                    && !target.carrier.chars().any(char::is_control)
                    && target.region.as_ref().is_none_or(|region| text(region, 64))
            })
    }
}

pub(crate) fn now_millis() -> Result<u64> {
    let value = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let value = u64::try_from(value)?;
    ensure!(value > 0, "UTC timestamp unavailable");
    Ok(value)
}

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub ip_version: IpVersion,
    pub count: u8,
    pub concurrency: u8,
    pub dns_timeout_ms: u64,
    pub connect_timeout_ms: u64,
    pub interval_ms: u64,
    pub total_timeout_ms: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Engine {
    pub name: String,
    pub version: String,
    pub source_commit: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Sample {
    pub attempted_at_ms: u64,
    pub elapsed_ms: f64,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Summary {
    pub attempted: usize,
    pub succeeded: usize,
    pub connection_success_percent: Option<f64>,
    pub latency_min_ms: Option<f64>,
    pub latency_mean_ms: Option<f64>,
    pub latency_max_ms: Option<f64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TargetResult {
    pub target: Target,
    pub address: Option<String>,
    pub dns_attempts: u8,
    pub status: String,
    pub samples: Vec<Sample>,
    pub summary: Summary,
    pub error: Option<String>,
    pub complete: bool,
}

impl TargetResult {
    pub(crate) fn queued(target: Target) -> Self {
        Self {
            target,
            address: None,
            dns_attempts: 0,
            status: "not_attempted".into(),
            samples: Vec::new(),
            summary: summary(&[]),
            error: None,
            complete: false,
        }
    }
    pub(crate) fn summarize(&mut self) {
        self.summary = summary(&self.samples);
    }
}

fn summary(samples: &[Sample]) -> Summary {
    let latencies: Vec<_> = samples
        .iter()
        .filter_map(|sample| sample.latency_ms)
        .collect();
    let succeeded = latencies.len();
    Summary {
        attempted: samples.len(),
        succeeded,
        connection_success_percent: (!samples.is_empty())
            .then(|| succeeded as f64 / samples.len() as f64 * 100.0),
        latency_min_ms: latencies.iter().copied().reduce(f64::min),
        latency_mean_ms: (!latencies.is_empty())
            .then(|| latencies.iter().sum::<f64>() / succeeded as f64),
        latency_max_ms: latencies.iter().copied().reduce(f64::max),
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub schema: u32,
    pub method: String,
    pub semantics: String,
    pub engine: Engine,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub parameters: Parameters,
    pub target_digest: String,
    pub targets: Vec<TargetResult>,
    pub complete: bool,
    pub deadline_exceeded: bool,
    pub upload_enabled: bool,
    pub ranking_enabled: bool,
    pub speedtest_enabled: bool,
}
