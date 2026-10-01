use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeNetwork {
    #[default]
    Other,
    Telecom,
    Unicom,
    Mobile,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeIpVersion {
    #[default]
    Auto,
    Ipv4,
    Ipv6,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeAuthorizationBasis {
    #[default]
    Unconfirmed,
    Owned,
    Permission,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProbeAuthorization {
    pub basis: ProbeAuthorizationBasis,
    pub confirmed: bool,
    pub source: String,
    pub scope: String,
    pub expires_at: Option<i64>,
}

impl ProbeAuthorization {
    pub fn confirmed_at(&self, now: i64) -> bool {
        self.confirmed
            && self.basis != ProbeAuthorizationBasis::Unconfirmed
            && !self.source.trim().is_empty()
            && !self.scope.trim().is_empty()
            && self.expires_at.is_none_or(|expires| expires > now)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProbeMonitoring {
    pub network: ProbeNetwork,
    pub region: String,
    pub ip_version: ProbeIpVersion,
    pub authorization: ProbeAuthorization,
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
    #[serde(default)]
    pub monitoring: ProbeMonitoring,
}

impl ProbeSpec {
    pub fn valid(&self) -> bool {
        let authorization = &self.monitoring.authorization;
        !self.name.trim().is_empty()
            && self.name.len() <= 128
            && self.carrier.len() <= 64
            && self.monitoring.region.len() <= 128
            && !self.monitoring.region.chars().any(char::is_control)
            && (self.monitoring.network == ProbeNetwork::Other
                || !self.monitoring.region.trim().is_empty())
            && authorization.source.len() <= 512
            && authorization.scope.len() <= 512
            && !authorization.source.chars().any(char::is_control)
            && !authorization.scope.chars().any(char::is_control)
            && (!authorization.confirmed
                || (authorization.basis != ProbeAuthorizationBasis::Unconfirmed
                    && !authorization.source.trim().is_empty()
                    && !authorization.scope.trim().is_empty()))
            && authorization.expires_at.is_none_or(|expires| expires > 0)
            && !self.target.is_empty()
            && self.target.len() <= 253
            && !self.target.starts_with('-')
            && self
                .target
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".:-".contains(&b))
            && (10..=3600).contains(&self.interval_secs)
            && match self.kind {
                ProbeKind::Tcp => self.port.is_some_and(|port| port > 0),
                ProbeKind::Icmp => self.port.is_none(),
            }
    }

    pub fn runnable(&self, now: i64) -> bool {
        self.enabled && self.valid() && self.monitoring.authorization.confirmed_at(now)
    }

    pub fn same_measurement(&self, previous: &Self) -> bool {
        self.kind == previous.kind
            && self.target == previous.target
            && self.port == previous.port
            && self.monitoring.network == previous.monitoring.network
            && self.monitoring.region == previous.monitoring.region
            && self.monitoring.ip_version == previous.monitoring.ip_version
    }

    pub fn normalize(&mut self) {
        self.name = self.name.trim().into();
        self.target = self.target.trim().into();
        self.carrier = self.carrier.trim().into();
        self.monitoring.region = self.monitoring.region.trim().into();
        self.monitoring.authorization.source = self.monitoring.authorization.source.trim().into();
        self.monitoring.authorization.scope = self.monitoring.authorization.scope.trim().into();
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
    pub ip_version: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attempts: Option<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_configuration_is_readable_but_never_authorized() {
        let legacy = serde_json::json!({"id":Uuid::nil(),"name":"TEST_ONLY","kind":"tcp","target":"127.0.0.1","port":443,"interval_secs":30,"carrier":"","enabled":true});
        let mut spec: ProbeSpec = serde_json::from_value(legacy).unwrap();
        assert!(spec.valid());
        assert!(!spec.runnable(1));
        spec.monitoring.authorization = ProbeAuthorization {
            basis: ProbeAuthorizationBasis::Owned,
            confirmed: true,
            source: "TEST_ONLY loopback fixture owner".into(),
            scope: "TEST_ONLY TCP to 127.0.0.1:443, four attempts every 30 seconds".into(),
            expires_at: Some(100),
        };
        assert!(spec.runnable(99));
        assert!(!spec.runnable(100));
        spec.monitoring.authorization.scope.clear();
        assert!(!spec.valid());
        assert!(!spec.runnable(1));
    }

    #[test]
    fn legacy_result_reserialization_keeps_the_original_digest_input() {
        let legacy = serde_json::json!({"id":Uuid::nil(),"probe_id":Uuid::nil(),"sampled_at":123,"latency_ms":null,"loss_percent":100.0,"error":"permission denied"});
        let result: ProbeResult = serde_json::from_value(legacy.clone()).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), legacy);
    }
}
