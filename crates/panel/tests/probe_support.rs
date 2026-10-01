#![forbid(unsafe_code)]

use serde_json::{Value, json};
use sinan_protocol::{
    ProbeAddressFamily, ProbeAuthorization, ProbeAuthorizationKind, ProbeMonitor, ProbeSpec,
};

#[allow(dead_code)]
pub fn authorize(spec: &mut ProbeSpec) {
    let identity = spec.identity();
    spec.monitor = Some(ProbeMonitor {
        network: sinan_protocol::ProbeNetwork::Other,
        region: String::new(),
        address_family: ProbeAddressFamily::Any,
        authorization: Some(ProbeAuthorization {
            kind: ProbeAuthorizationKind::Owned,
            source: "TEST_ONLY isolated fixture inventory".into(),
            scope: "Only this exact fixture target and method; never public traffic".into(),
            enabled: true,
            expires_at: None,
            identity,
        }),
    });
}

#[allow(dead_code)]
pub fn authorized(value: Value) -> Value {
    let mut spec: ProbeSpec = serde_json::from_value(value).expect("valid fixture probe shape");
    authorize(&mut spec);
    json!(spec)
}
