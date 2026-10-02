use super::{ProbeAuthorization, ProbeSpec};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

pub const PROBE_LEASE_CAPABILITY: &str = "probe:authorized-lease";
pub const MAX_PROBE_LEASE_SECS: i64 = 90;

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
                    && probe.spec.runnable_at(self.issued_at)
                    && probe.spec.execution_authorized.is_none()
                    && probe
                        .spec
                        .monitor
                        .as_ref()
                        .and_then(|monitor| monitor.authorization.as_ref())
                        == Some(&probe.authorization)
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
