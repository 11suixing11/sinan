#![forbid(unsafe_code)]

use serde_json::json;
use sinan_protocol::{
    AuthorizedProbe, ProbeAuthorization, ProbeExecution, ProbeKind, ProbeLease, ProbeResult,
    ProbeScope, ProbeSpec,
};
use uuid::Uuid;

const LEGACY_SPEC: &str = r#"{"id":"00000000-0000-0000-0000-000000000003","name":"fixture","kind":"tcp","target":"127.0.0.1","port":443,"interval_secs":10,"carrier":"","enabled":true}"#;
const LEGACY_RESULT: &str = r#"{"id":"00000000-0000-0000-0000-000000000004","probe_id":"00000000-0000-0000-0000-000000000003","sampled_at":1790000000000,"latency_ms":0.0,"loss_percent":0.0,"error":null}"#;

fn authorized() -> AuthorizedProbe {
    AuthorizedProbe {
        spec: serde_json::from_str(LEGACY_SPEC).unwrap(),
        authorization: ProbeAuthorization {
            region: "fixture-region".into(),
            source: "operator-owned fixture".into(),
            scope: ProbeScope::Owned,
            evidence: "operator controls the local fixture".into(),
            expires_at: None,
        },
    }
}

fn lease() -> ProbeLease {
    ProbeLease {
        id: Uuid::from_u128(5),
        server_id: 7,
        revision: 1,
        issued_at: 1_790_000_000,
        expires_at: 1_790_000_090,
        probes: vec![authorized()],
    }
}

#[test]
fn legacy_probe_vector_and_results_keep_their_exact_wire_bytes() {
    let spec: ProbeSpec = serde_json::from_str(LEGACY_SPEC).unwrap();
    assert!(spec.valid());
    assert_eq!(
        serde_json::to_string(&vec![spec]).unwrap(),
        format!("[{LEGACY_SPEC}]")
    );
    let result: ProbeResult = serde_json::from_str(LEGACY_RESULT).unwrap();
    assert!(result.execution.is_none());
    assert_eq!(serde_json::to_string(&result).unwrap(), LEGACY_RESULT);
    let mut unknown = serde_json::from_str::<serde_json::Value>(LEGACY_SPEC).unwrap();
    unknown["authorization"] = json!(authorized().authorization);
    assert!(serde_json::from_value::<ProbeSpec>(unknown).is_err());
}

#[test]
fn authorization_scopes_require_bounded_evidence_and_explicit_expiry() {
    for scope in [ProbeScope::Owned, ProbeScope::ThirdParty] {
        let mut permission = authorized().authorization;
        permission.scope = scope;
        permission.expires_at = Some(200);
        assert!(permission.allows(199));
        assert!(!permission.allows(200));
        assert!(!permission.allows(201));
        let wire = serde_json::to_value(&permission).unwrap();
        assert_eq!(
            wire["scope"],
            if scope == ProbeScope::Owned {
                "owned"
            } else {
                "third_party"
            }
        );
        assert_eq!(
            serde_json::from_value::<ProbeAuthorization>(wire).unwrap(),
            permission
        );
        for field in ["source", "evidence"] {
            let mut wire = serde_json::to_value(&permission).unwrap();
            wire[field] = json!("   ");
            assert!(
                !serde_json::from_value::<ProbeAuthorization>(wire)
                    .unwrap()
                    .valid()
            );
        }
    }
    for (field, value) in [
        ("region", json!("r".repeat(65))),
        ("source", json!("s".repeat(257))),
        ("evidence", json!("e".repeat(513))),
        ("source", json!("source\ncontrol")),
        ("evidence", json!("evidence\0control")),
        ("expires_at", json!(0)),
        ("expires_at", json!(-1)),
    ] {
        let mut wire = serde_json::to_value(authorized().authorization).unwrap();
        wire[field] = value;
        assert!(
            !serde_json::from_value::<ProbeAuthorization>(wire)
                .unwrap()
                .valid(),
            "{field}"
        );
    }
    let mut wire = serde_json::to_value(authorized().authorization).unwrap();
    wire.as_object_mut().unwrap().remove("region");
    assert!(
        serde_json::from_value::<ProbeAuthorization>(wire)
            .unwrap()
            .region
            .is_empty()
    );
}

#[test]
fn lease_lifetime_server_identity_and_revision_limits_are_checked() {
    let original = lease();
    assert!(original.valid());
    assert_eq!(sinan_protocol::MAX_PROBE_LEASE_SECS, 90);
    assert_eq!(
        sinan_protocol::PROBE_LEASE_CAPABILITY,
        "probe:authorized-lease"
    );
    for lifetime in [1, 90] {
        let mut changed = original.clone();
        changed.expires_at = changed.issued_at + lifetime;
        assert!(changed.valid());
    }
    for lifetime in [-1, 0, 91] {
        let mut changed = original.clone();
        changed.expires_at = changed.issued_at + lifetime;
        assert!(!changed.valid());
    }
    for case in 0..5 {
        let mut changed = original.clone();
        match case {
            0 => changed.id = Uuid::nil(),
            1 => changed.server_id = 0,
            2 => changed.issued_at = 0,
            3 => changed.revision = i64::MAX as u64 + 1,
            4 => {
                changed.issued_at = i64::MIN;
                changed.expires_at = i64::MAX;
            }
            _ => unreachable!(),
        }
        assert!(!changed.valid(), "case {case}");
    }
    let mut maximum = original;
    maximum.revision = i64::MAX as u64;
    assert!(maximum.valid());
}

#[test]
fn leases_reject_duplicate_targets_disabled_targets_and_permissions_ending_too_soon() {
    let mut value = lease();
    value.probes.clear();
    assert!(
        value.valid(),
        "an empty replacement must be able to stop all work"
    );
    for id in 1..=32 {
        let mut target = authorized();
        target.spec.id = Uuid::from_u128(id);
        value.probes.push(target);
    }
    assert!(value.valid());
    let mut too_many = value.clone();
    let mut additional = authorized();
    additional.spec.id = Uuid::from_u128(33);
    too_many.probes.push(additional);
    assert!(!too_many.valid());
    for case in 0..6 {
        let mut changed = value.clone();
        match case {
            0 => changed.probes[1].spec.id = changed.probes[0].spec.id,
            1 => changed.probes[0].spec.id = Uuid::nil(),
            2 => changed.probes[0].spec.enabled = false,
            3 => changed.probes[0].spec.port = Some(0),
            4 => changed.probes[0].authorization.expires_at = Some(changed.issued_at),
            5 => changed.probes[0].authorization.expires_at = Some(changed.expires_at - 1),
            _ => unreachable!(),
        }
        assert!(!changed.valid(), "case {case}");
    }
    value.probes[0].authorization.expires_at = Some(value.expires_at);
    assert!(value.valid());
}

#[test]
fn execution_records_preserve_the_authorized_target_without_changing_legacy_results() {
    let snapshot = lease();
    let execution = ProbeExecution {
        lease_id: snapshot.id,
        revision: snapshot.revision,
        issued_at: snapshot.issued_at,
        expires_at: snapshot.expires_at,
        probe: snapshot.probes[0].clone(),
    };
    assert!(execution.valid());
    let mut result: ProbeResult = serde_json::from_str(LEGACY_RESULT).unwrap();
    result.execution = Some(execution.clone());
    let wire = serde_json::to_value(&result).unwrap();
    assert_eq!(wire["execution"]["probe"]["spec"]["kind"], "tcp");
    assert_eq!(
        wire["execution"]["probe"]["authorization"]["scope"],
        "owned"
    );
    assert_eq!(serde_json::from_value::<ProbeResult>(wire).unwrap(), result);
    let mut changed = execution;
    changed.probe.spec.kind = ProbeKind::Icmp;
    assert!(
        !changed.valid(),
        "the exact TCP port must not be accepted as ICMP"
    );
}

#[test]
fn lease_and_authorization_records_reject_unknown_or_missing_control_fields() {
    for level in ["lease", "probe", "authorization", "execution"] {
        let mut value = serde_json::to_value(lease()).unwrap();
        match level {
            "lease" => {
                value["unexpected"] = json!(true);
                assert!(serde_json::from_value::<ProbeLease>(value).is_err());
            }
            "probe" => {
                value["probes"][0]["unexpected"] = json!(true);
                assert!(serde_json::from_value::<ProbeLease>(value).is_err());
            }
            "authorization" => {
                value["probes"][0]["authorization"]["unexpected"] = json!(true);
                assert!(serde_json::from_value::<ProbeLease>(value).is_err());
            }
            "execution" => {
                let mut execution = json!({"lease_id":value["id"],"revision":1,"issued_at":value["issued_at"],"expires_at":value["expires_at"],"probe":value["probes"][0]});
                execution["unexpected"] = json!(true);
                assert!(serde_json::from_value::<ProbeExecution>(execution).is_err());
            }
            _ => unreachable!(),
        }
    }
    for field in [
        "id",
        "server_id",
        "revision",
        "issued_at",
        "expires_at",
        "probes",
    ] {
        let mut value = serde_json::to_value(lease()).unwrap();
        value.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<ProbeLease>(value).is_err(),
            "{field}"
        );
    }
}
