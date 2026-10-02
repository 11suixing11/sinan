#![forbid(unsafe_code)]

use serde_json::json;
use sinan_protocol::{
    AuthorizedProbe, ProbeAddressFamily, ProbeAuthorization, ProbeAuthorizationKind,
    ProbeExecution, ProbeLease, ProbeMonitor, ProbeNetwork, ProbeResult, ProbeSpec,
};
use uuid::Uuid;

const LEGACY_SPEC: &str = r#"{"id":"00000000-0000-0000-0000-000000000003","name":"fixture","kind":"tcp","target":"127.0.0.1","port":443,"interval_secs":10,"carrier":"","enabled":true}"#;
const LEGACY_RESULT: &str = r#"{"id":"00000000-0000-0000-0000-000000000004","probe_id":"00000000-0000-0000-0000-000000000003","sampled_at":1790000000000,"latency_ms":0.0,"loss_percent":0.0,"error":null}"#;

fn lease() -> ProbeLease {
    let mut spec: ProbeSpec = serde_json::from_str(LEGACY_SPEC).unwrap();
    let authorization = ProbeAuthorization {
        kind: ProbeAuthorizationKind::Owned,
        source: "TEST_ONLY operator-owned loopback".into(),
        scope: "Only this fixture endpoint and method".into(),
        enabled: true,
        expires_at: None,
        identity: spec.identity(),
    };
    spec.monitor = Some(ProbeMonitor {
        network: ProbeNetwork::Other,
        region: "fixture region".into(),
        address_family: ProbeAddressFamily::Any,
        authorization: Some(authorization.clone()),
    });
    ProbeLease {
        id: Uuid::from_u128(5),
        server_id: 7,
        revision: 1,
        issued_at: 1_790_000_000,
        expires_at: 1_790_000_090,
        probes: vec![AuthorizedProbe {
            spec,
            authorization,
        }],
    }
}

#[test]
fn legacy_probe_vector_and_results_keep_their_exact_wire_bytes() {
    let spec: ProbeSpec = serde_json::from_str(LEGACY_SPEC).unwrap();
    assert!(spec.valid());
    assert!(!spec.authorized_at(1_790_000_000));
    assert_eq!(
        serde_json::to_string(&vec![spec]).unwrap(),
        format!("[{LEGACY_SPEC}]")
    );
    let result: ProbeResult = serde_json::from_str(LEGACY_RESULT).unwrap();
    assert!(result.execution.is_none());
    assert_eq!(serde_json::to_string(&result).unwrap(), LEGACY_RESULT);
}

#[test]
fn lease_lifetime_device_identity_and_revision_limits_are_checked() {
    let original = lease();
    assert!(original.valid());
    assert_eq!(sinan_protocol::MAX_PROBE_LEASE_SECS, 90);
    for seconds in [1, 90] {
        let mut changed = original.clone();
        changed.expires_at = changed.issued_at + seconds;
        assert!(changed.valid());
    }
    for seconds in [-1, 0, 91] {
        let mut changed = original.clone();
        changed.expires_at = changed.issued_at + seconds;
        assert!(!changed.valid());
    }
    for (field, value) in [
        ("id", json!(Uuid::nil())),
        ("server_id", json!(0)),
        ("revision", json!(u64::MAX)),
        ("issued_at", json!(0)),
    ] {
        let mut changed = json!(original);
        changed[field] = value;
        assert!(
            !serde_json::from_value::<ProbeLease>(changed)
                .unwrap()
                .valid(),
            "{field}"
        );
    }
}

#[test]
fn exact_main_monitor_authorization_is_required_and_cannot_be_retargeted() {
    for property in ["source", "scope", "enabled", "target", "expiry", "monitor"] {
        let mut changed = lease();
        let probe = &mut changed.probes[0];
        match property {
            "source" => probe.authorization.source = "other source".into(),
            "scope" => probe.authorization.scope = "other scope".into(),
            "enabled" => probe.spec.enabled = false,
            "target" => probe.spec.target = "127.0.0.2".into(),
            "expiry" => probe.authorization.expires_at = Some(changed.expires_at - 1),
            _ => probe.spec.monitor = None,
        }
        assert!(!changed.valid(), "{property}");
    }
    let mut changed = lease();
    changed.probes[0].authorization.expires_at = Some(changed.expires_at);
    changed.probes[0]
        .spec
        .monitor
        .as_mut()
        .unwrap()
        .authorization = Some(changed.probes[0].authorization.clone());
    assert!(changed.valid());
    changed.expires_at += 1;
    assert!(!changed.valid());
}

#[test]
fn duplicate_targets_bounds_unknown_fields_and_execution_context_are_checked() {
    let mut changed = lease();
    changed.probes.push(changed.probes[0].clone());
    assert!(!changed.valid());
    changed.probes.clear();
    assert!(changed.valid());
    let original = lease();
    let execution = ProbeExecution {
        lease_id: original.id,
        revision: original.revision,
        issued_at: original.issued_at,
        expires_at: original.expires_at,
        probe: original.probes[0].clone(),
    };
    assert!(execution.valid());
    for field in ["lease", "probe", "authorization", "execution"] {
        let mut wire = json!(original);
        match field {
            "lease" => wire["unexpected"] = json!(true),
            "probe" => wire["probes"][0]["unexpected"] = json!(true),
            "authorization" => wire["probes"][0]["authorization"]["unexpected"] = json!(true),
            _ => {
                let mut wire = json!(execution);
                wire["unexpected"] = json!(true);
                assert!(serde_json::from_value::<ProbeExecution>(wire).is_err());
                continue;
            }
        }
        assert!(
            serde_json::from_value::<ProbeLease>(wire).is_err(),
            "{field}"
        );
    }
    changed = original.clone();
    changed.probes = (1..=33)
        .map(|id| {
            let mut probe = original.probes[0].clone();
            probe.spec.id = Uuid::from_u128(id);
            probe
        })
        .collect();
    assert!(!changed.valid());
}
