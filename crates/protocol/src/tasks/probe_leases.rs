use super::ProbeSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub const PROBE_LEASE_CAPABILITY: &str = "probe:authorized-lease";
pub const MAX_PROBE_LEASE_SECS: i64 = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeScope {
    Owned,
    ThirdParty,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeAuthorization {
    #[serde(default)]
    pub region: String,
    pub source: String,
    pub scope: ProbeScope,
    pub evidence: String,
    pub expires_at: Option<i64>,
}

impl ProbeAuthorization {
    pub fn valid(&self) -> bool {
        fn bounded(value: &str, maximum: usize, required: bool) -> bool {
            (!required || !value.trim().is_empty())
                && value.len() <= maximum
                && !value.chars().any(char::is_control)
        }
        bounded(&self.region, 64, false)
            && bounded(&self.source, 256, true)
            && bounded(&self.evidence, 512, true)
            && self.expires_at.is_none_or(|expires| expires > 0)
    }

    pub fn allows(&self, at: i64) -> bool {
        self.valid() && self.expires_at.is_none_or(|expires| expires > at)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizedProbe {
    pub spec: ProbeSpec,
    pub authorization: ProbeAuthorization,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeLease {
    pub id: Uuid,
    pub server_id: i64,
    pub revision: u64,
    pub issued_at: i64,
    pub expires_at: i64,
    pub probes: Vec<AuthorizedProbe>,
}

impl ProbeLease {
    pub fn valid(&self) -> bool {
        let lifetime = self.expires_at.checked_sub(self.issued_at);
        let mut ids = HashSet::new();
        !self.id.is_nil()
            && self.server_id > 0
            && self.revision <= i64::MAX as u64
            && self.issued_at > 0
            && lifetime.is_some_and(|seconds| (1..=MAX_PROBE_LEASE_SECS).contains(&seconds))
            && self.probes.len() <= 32
            && self.probes.iter().all(|probe| {
                !probe.spec.id.is_nil()
                    && ids.insert(probe.spec.id)
                    && probe.spec.valid()
                    && probe.spec.enabled
                    && probe.authorization.allows(self.issued_at)
                    && probe
                        .authorization
                        .expires_at
                        .is_none_or(|expires| expires >= self.expires_at)
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeExecution {
    pub lease_id: Uuid,
    pub revision: u64,
    pub issued_at: i64,
    pub expires_at: i64,
    pub probe: AuthorizedProbe,
}

impl ProbeExecution {
    pub fn valid(&self) -> bool {
        ProbeLease {
            id: self.lease_id,
            server_id: 1,
            revision: self.revision,
            issued_at: self.issued_at,
            expires_at: self.expires_at,
            probes: vec![self.probe.clone()],
        }
        .valid()
    }
}
