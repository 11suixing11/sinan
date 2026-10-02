//! Deterministic ordered paths; imported routing and identities never enter the public view.

use crate::{
    CompileError, Node, Relay,
    external::{ExternalCapabilities, ExternalOutbound, ExternalTransport},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub const PATH_CAPABILITY: &str = "runtime:dependency-validation:v1";
pub const CONTROL_PORT: u16 = 18086;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Hop {
    Managed {
        server_id: i64,
        endpoint: Box<Node>,
        identity: Uuid,
    },
    External {
        node_id: i64,
        version_id: i64,
        outbound: ExternalOutbound,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Path {
    pub chain_id: i64,
    pub generation: u64,
    pub entry_server_id: i64,
    pub entry_node_id: i64,
    pub active: bool,
    pub hops: Vec<Hop>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Networks {
    pub tcp: bool,
    pub udp: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Control {
    pub secret: String,
    pub test_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathCheck {
    pub scope: String,
    pub generation: u64,
    pub outbound: String,
    pub url: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Constraints {
    pub schema: u32,
    pub active: BTreeMap<String, u64>,
    pub retired: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct Compiled {
    pub config: String,
    pub checks: Vec<PathCheck>,
    pub constraints: Constraints,
    pub features: Vec<String>,
}

fn invalid(path: &Path, reason: &str) -> CompileError {
    CompileError::InvalidNode {
        node_id: path.entry_node_id,
        reason: reason.into(),
    }
}

impl Hop {
    fn capabilities(&self) -> Result<ExternalCapabilities, CompileError> {
        match self {
            Self::Managed {
                endpoint, identity, ..
            } => ExternalOutbound(managed_outbound(endpoint, *identity))
                .capabilities()
                .map_err(|_| CompileError::InvalidNode {
                    node_id: endpoint.id,
                    reason: "unsupported managed path endpoint".into(),
                }),
            Self::External { outbound, .. } => {
                outbound
                    .capabilities()
                    .map_err(|_| CompileError::InvalidNode {
                        node_id: 0,
                        reason: "unsupported imported path endpoint".into(),
                    })
            }
        }
    }

    fn address(&self) -> (String, u16) {
        match self {
            Self::Managed { endpoint, .. } => (
                crate::unbracket_host(&endpoint.public_host).to_ascii_lowercase(),
                endpoint.public_port(),
            ),
            Self::External { outbound, .. } => (
                crate::unbracket_host(outbound.server()).to_ascii_lowercase(),
                outbound.port(),
            ),
        }
    }
}

fn managed_outbound(endpoint: &Node, identity: Uuid) -> Value {
    let mut value = crate::protocols::client(
        endpoint,
        &crate::Access {
            user_id: 0,
            uuid: identity,
            credential: String::new(),
        },
    );
    value
        .as_object_mut()
        .expect("managed outbound")
        .remove("tag");
    value
}

pub fn validate(path: &Path) -> Result<Networks, CompileError> {
    if path.chain_id <= 0
        || path.generation == 0
        || path.entry_server_id <= 0
        || path.entry_node_id <= 0
        || path.hops.is_empty()
        || path.hops.len() > 8
    {
        return Err(invalid(
            path,
            "a path requires one to eight ordered hops and positive identities",
        ));
    }
    let mut servers = BTreeSet::from([path.entry_server_id]);
    let mut resources = BTreeSet::new();
    let mut endpoints = BTreeSet::new();
    let mut capabilities = Vec::new();
    for hop in &path.hops {
        match hop {
            Hop::Managed {
                server_id,
                endpoint,
                identity,
            } => {
                if *server_id <= 0
                    || !servers.insert(*server_id)
                    || endpoint.id == path.entry_node_id
                    || !endpoint.enabled
                    || !endpoint.protocol_config.is_reality()
                    || identity.is_nil()
                    || !resources.insert(format!("managed:{}", endpoint.id))
                {
                    return Err(invalid(
                        path,
                        "managed hops must be distinct enabled Reality endpoints on other servers",
                    ));
                }
                crate::validate_node(endpoint)?;
            }
            Hop::External {
                node_id,
                version_id,
                outbound,
            } => {
                if *node_id <= 0
                    || *version_id <= 0
                    || !resources.insert(format!("external:{node_id}"))
                {
                    return Err(invalid(
                        path,
                        "imported hop identities must be positive and distinct",
                    ));
                }
                outbound
                    .validate()
                    .map_err(|_| invalid(path, "invalid imported endpoint configuration"))?;
            }
        }
        if !endpoints.insert(hop.address()) {
            return Err(invalid(path, "a path cannot revisit the same endpoint"));
        }
        capabilities.push(hop.capabilities()?);
    }
    let carries = |mut network: ExternalTransport| {
        for capability in capabilities.iter().rev() {
            if !capability.carries(network) {
                return false;
            }
            network = match network {
                ExternalTransport::Tcp => capability.required_transport,
                ExternalTransport::Udp => capability.udp_transport,
            };
        }
        true
    };
    let networks = Networks {
        tcp: carries(ExternalTransport::Tcp),
        udp: carries(ExternalTransport::Udp),
    };
    if !networks.tcp {
        return Err(invalid(
            path,
            "the ordered lower transports cannot carry a TCP connection through every hop",
        ));
    }
    Ok(networks)
}

pub fn scope(chain_id: i64) -> String {
    format!("path-{chain_id}")
}

pub fn tag(chain_id: i64, generation: u64, position: usize) -> String {
    format!("path-{chain_id}-g{generation}-h{position}")
}

/// Both active and candidate generations are compiled from immutable endpoint snapshots.
pub fn compile(
    server_id: i64,
    nodes: &[Node],
    legacy: &[Relay],
    paths: &[Path],
    blocked_entries: &[i64],
    retired: BTreeMap<String, u64>,
    control: Option<&Control>,
) -> Result<Compiled, CompileError> {
    let mut sorted: Vec<_> = paths.iter().collect();
    sorted.sort_by_key(|path| (path.chain_id, path.generation));
    let mut seen = BTreeSet::new();
    let mut active_entries = BTreeSet::new();
    let mut shadow_nodes = nodes.to_vec();
    for node in &mut shadow_nodes {
        if blocked_entries.contains(&node.id) {
            node.users.clear();
        }
    }
    let original = crate::compile_server_with_relays(&shadow_nodes, legacy)?;
    let mut config: Value = serde_json::from_str(&original)?;
    if config["route"]["rules"].is_null() {
        config["route"]["rules"] = json!([]);
    }
    let mut checks = Vec::new();
    let mut constraints = Constraints {
        schema: 1,
        retired,
        ..Constraints::default()
    };
    let mut needs_dns = false;
    let mut features = BTreeSet::new();
    for path in sorted {
        let networks = validate(path)?;
        if !seen.insert((path.chain_id, path.generation)) {
            return Err(invalid(path, "duplicate path generation"));
        }
        let mut present = false;
        for (position, hop) in path.hops.iter().enumerate() {
            if let Hop::Managed {
                server_id: host,
                endpoint,
                identity,
            } = hop
                && *host == server_id
            {
                present = true;
                let identity = crate::protocols::reality_identity(
                    endpoint,
                    format!("relay_{}_g{}_h{}", path.chain_id, path.generation, position),
                    *identity,
                );
                let inbounds = config["inbounds"]
                    .as_array_mut()
                    .expect("compiled inbounds");
                let inbound_tag = format!("node-{}", endpoint.id);
                if let Some(inbound) = inbounds
                    .iter_mut()
                    .find(|inbound| inbound["tag"] == inbound_tag)
                {
                    // Endpoint changes cannot silently replace the live listening configuration.
                    let mut expected = crate::protocols::server(endpoint, &[]);
                    let mut actual = inbound.clone();
                    expected
                        .as_object_mut()
                        .expect("inbound object")
                        .remove("users");
                    actual
                        .as_object_mut()
                        .expect("inbound object")
                        .remove("users");
                    if expected != actual {
                        return Err(invalid(
                            path,
                            "managed endpoint differs from its immutable listening snapshot",
                        ));
                    }
                    inbound["users"]
                        .as_array_mut()
                        .expect("inbound identities")
                        .push(identity);
                } else {
                    let mut inbound = crate::protocols::server(endpoint, &[]);
                    inbound["users"] = json!([identity]);
                    inbounds.push(inbound);
                }
            }
        }
        if server_id != path.entry_server_id {
            if present {
                constraints
                    .active
                    .entry(scope(path.chain_id))
                    .and_modify(|value| *value = (*value).max(path.generation))
                    .or_insert(path.generation);
            }
            continue;
        }
        let Some(entry) = nodes.iter().find(|node| node.id == path.entry_node_id) else {
            return Err(invalid(
                path,
                "path entry is absent from its server snapshot",
            ));
        };
        if !entry.enabled || !entry.protocol_config.is_reality() {
            return Err(invalid(path, "path entry must be enabled Reality"));
        }
        if path.hops.iter().any(|hop| {
            hop.address()
                == (
                    crate::unbracket_host(&entry.public_host).to_ascii_lowercase(),
                    entry.public_port(),
                )
        }) {
            return Err(invalid(path, "path returns to its public entry endpoint"));
        }
        let mut previous = None;
        for (position, hop) in path.hops.iter().enumerate() {
            features.extend(hop.capabilities()?.required_features);
            let outbound_tag = tag(path.chain_id, path.generation, position);
            let outbound = match hop {
                Hop::Managed {
                    endpoint, identity, ..
                } => ExternalOutbound(managed_outbound(endpoint, *identity)),
                Hop::External { outbound, .. } => outbound.clone(),
            };
            needs_dns |= outbound.server().parse::<std::net::IpAddr>().is_err();
            let rendered = outbound
                .render(&outbound_tag, previous.as_deref(), Some("path-bootstrap"))
                .map_err(|_| invalid(path, "path outbound cannot be compiled"))?;
            config["outbounds"]
                .as_array_mut()
                .expect("compiled outbounds")
                .push(rendered);
            previous = Some(outbound_tag);
        }
        let final_tag = previous.expect("validated nonempty path");
        if path.active {
            if !active_entries.insert(entry.id) {
                return Err(invalid(
                    path,
                    "a public entry cannot route to two active generations",
                ));
            }
            constraints
                .active
                .insert(scope(path.chain_id), path.generation);
            if !networks.udp {
                config["route"]["rules"]
                    .as_array_mut()
                    .expect("path rules")
                    .push(json!({
                    "inbound":[format!("node-{}",entry.id)],"network":"udp","action":"reject"}));
            }
            config["route"]["rules"]
                .as_array_mut()
                .expect("path rules")
                .push(json!({
                "inbound":[format!("node-{}",entry.id)],"action":"route","outbound":final_tag}));
        }
        let control = control.ok_or_else(|| {
            invalid(
                path,
                "path verification requires a protected local control configuration",
            )
        })?;
        checks.push(PathCheck {
            scope: scope(path.chain_id),
            generation: path.generation,
            outbound: final_tag,
            url: control.test_url.clone(),
        });
    }
    if !checks.is_empty() {
        features.insert("with_clash_api".into());
        let control = control.expect("checked control configuration");
        if control.secret.len() < 32
            || control.secret.len() > 256
            || !control
                .secret
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
            || !control.test_url.starts_with("https://")
            || control.test_url.len() > 2048
            || control.test_url.chars().any(char::is_control)
            || nodes.iter().any(|node| node.port == CONTROL_PORT)
        {
            return Err(CompileError::InvalidNode {
                node_id: 0,
                reason: "invalid protected path control configuration".into(),
            });
        }
        config["experimental"]["clash_api"] = json!({"external_controller":format!("127.0.0.1:{CONTROL_PORT}"),"secret":control.secret,
            "access_control_allow_origin":["http://127.0.0.1"],"access_control_allow_private_network":false});
    }
    if needs_dns {
        config["dns"] = json!({"servers":[{"type":"local","tag":"path-bootstrap"}]});
    }
    if checks.len() > 128
        || serde_json::to_vec(&checks)?.len() > 64 * 1024
        || constraints.active.len() + constraints.retired.len() > 1024
        || serde_json::to_vec(&constraints)?.len() > 64 * 1024
    {
        return Err(CompileError::InvalidNode {
            node_id: 0,
            reason: "path verification or recovery metadata exceeds its runtime budget".into(),
        });
    }
    Ok(Compiled {
        config: if paths.is_empty() && blocked_entries.is_empty() {
            original
        } else {
            crate::pretty(config)?
        },
        checks,
        constraints,
        features: features.into_iter().collect(),
    })
}
