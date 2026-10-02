use super::{OrderedPath, PathHop};
use crate::{
    CompileError,
    external::{NormalizedOutbound, Transport},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathCapabilities {
    pub tcp: bool,
    pub udp: bool,
}

impl PathCapabilities {
    fn both() -> Self {
        Self {
            tcp: true,
            udp: true,
        }
    }
    fn tcp() -> Self {
        Self {
            tcp: true,
            udp: false,
        }
    }
    fn udp() -> Self {
        Self {
            tcp: false,
            udp: true,
        }
    }
    fn includes(self, other: Self) -> bool {
        (!other.tcp || self.tcp) && (!other.udp || self.udp)
    }
}

fn payload(hop: &PathHop) -> PathCapabilities {
    match hop {
        PathHop::Managed { .. } => PathCapabilities::both(),
        PathHop::External { outbound, .. } => PathCapabilities {
            tcp: outbound.tcp(),
            udp: outbound.udp(),
        },
    }
}

// Work backward from payload to carrier. UDP encapsulated inside VLESS/AnyTLS
// still needs only a TCP carrier; native SS/SOCKS datagrams need UDP as well.
fn carrier(hop: &PathHop, requested: PathCapabilities) -> PathCapabilities {
    let PathHop::External { outbound, .. } = hop else {
        return PathCapabilities::tcp();
    };
    if matches!(
        outbound.as_ref(),
        NormalizedOutbound::Hysteria2 { .. } | NormalizedOutbound::Tuic { .. }
    ) || matches!(
        outbound.common().transport.as_ref(),
        Some(Transport::Quic {})
    ) {
        return PathCapabilities::udp();
    }
    let uot = outbound
        .common()
        .udp_over_tcp
        .as_ref()
        .is_some_and(|uot| uot.enabled);
    let mux = outbound
        .common()
        .multiplex
        .as_ref()
        .is_some_and(|mux| mux.enabled);
    if super::outbound_validation::plugin_quic(outbound) {
        return PathCapabilities::udp();
    }
    match outbound.as_ref() {
        NormalizedOutbound::Shadowsocks { .. } if requested.udp && !uot && !mux => requested,
        NormalizedOutbound::Socks { .. } if requested.udp && !uot => PathCapabilities::both(),
        _ => PathCapabilities::tcp(),
    }
}

fn carries(path: &OrderedPath, requested: PathCapabilities) -> bool {
    let mut needed = requested;
    for hop in path.hops.iter().rev() {
        if !payload(hop).includes(needed) {
            return false;
        }
        needed = carrier(hop, needed);
    }
    true
}

pub fn path_capabilities(path: &OrderedPath) -> Result<PathCapabilities, CompileError> {
    super::validate_path(path)?;
    computed_capabilities(path)
}

pub(super) fn computed_capabilities(path: &OrderedPath) -> Result<PathCapabilities, CompileError> {
    let capabilities = PathCapabilities {
        tcp: carries(path, PathCapabilities::tcp()),
        udp: carries(path, PathCapabilities::udp()),
    };
    if !capabilities.tcp && !capabilities.udp {
        return Err(CompileError::InvalidPath {
            chain_id: path.chain_id,
            position: None,
            reason: "a preceding hop cannot carry the next hop's required transport",
        });
    }
    Ok(capabilities)
}

/// Required tags are requirements, never observations of the installed binary.
pub fn required_build_tags(path: &OrderedPath) -> Vec<String> {
    let mut tags = BTreeSet::from(["with_v2ray_api", "with_clash_api"]);
    for hop in &path.hops {
        match hop {
            PathHop::Managed { .. } => {
                tags.insert("with_utls");
            }
            PathHop::External { outbound, .. } => {
                if matches!(
                    outbound.as_ref(),
                    NormalizedOutbound::Hysteria2 { .. } | NormalizedOutbound::Tuic { .. }
                ) || matches!(
                    outbound.common().transport.as_ref(),
                    Some(Transport::Quic {})
                ) || super::outbound_validation::plugin_quic(outbound)
                {
                    tags.insert("with_quic");
                }
                if matches!(
                    outbound.common().transport.as_ref(),
                    Some(Transport::Grpc { .. })
                ) {
                    tags.insert("with_grpc");
                }
                if outbound
                    .common()
                    .tls
                    .as_ref()
                    .is_some_and(|tls| tls.utls.as_ref().is_some_and(|utls| utls.enabled))
                {
                    tags.insert("with_utls");
                }
            }
        }
    }
    tags.into_iter().map(str::to_owned).collect()
}
