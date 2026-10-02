use crate::Metrics;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TelemetrySample {
    pub id: Uuid,
    pub sampled_at: i64,
    pub metrics: Metrics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TelemetryBatch {
    pub samples: Vec<TelemetrySample>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelemetryAck {
    pub ids: Vec<Uuid>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentSettings {
    pub sample_interval_secs: u64,
    pub upload_interval_secs: u64,
    pub auto_update: bool,
    pub discover_public_ips: bool,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            sample_interval_secs: 1,
            upload_interval_secs: 3,
            auto_update: false,
            discover_public_ips: true,
        }
    }
}

impl AgentSettings {
    pub fn valid(&self) -> bool {
        (1..=60).contains(&self.sample_interval_secs)
            && (self.sample_interval_secs..=60).contains(&self.upload_interval_secs)
    }
}

/// Separate from AgentSettings so older strict decoders remain compatible.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TelemetrySettings {
    pub persist_interval_secs: u64,
}

impl Default for TelemetrySettings {
    fn default() -> Self {
        Self {
            persist_interval_secs: 60,
        }
    }
}

impl TelemetrySettings {
    pub fn valid(&self) -> bool {
        (15..=3600).contains(&self.persist_interval_secs)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: Option<u64>,
    pub used_bytes: Option<u64>,
    pub read_bytes_per_sec: Option<f64>,
    pub write_bytes_per_sec: Option<f64>,
    pub read_iops: Option<f64>,
    pub write_iops: Option<f64>,
    pub await_ms: Option<f64>,
    pub utilization_percent: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GpuMetrics {
    pub model: String,
    pub usage_percent: Option<f64>,
    pub memory_used: Option<u64>,
    pub memory_total: Option<u64>,
}

pub fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|value| value.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persistence_settings_do_not_expand_the_legacy_agent_settings_message() {
        let legacy = serde_json::to_value(AgentSettings::default()).unwrap();
        assert!(legacy.get("persist_interval_secs").is_none());
        assert!(serde_json::from_value::<AgentSettings>(legacy).is_ok());
        assert_eq!(TelemetrySettings::default().persist_interval_secs, 60);
        for interval in [0, 14, 3601, u64::MAX] {
            assert!(
                !TelemetrySettings {
                    persist_interval_secs: interval
                }
                .valid()
            );
        }
        for interval in [15, 60, 3600] {
            assert!(
                TelemetrySettings {
                    persist_interval_secs: interval
                }
                .valid()
            );
        }
    }
}
