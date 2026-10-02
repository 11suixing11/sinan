use super::model::acceptance_name;
use super::{ManagedAcceptance, OrderedPath, PathHop, ProbeControl};
use crate::{Access, CompileError, Node, Relay};
use serde_json::{Value, json};
use std::collections::BTreeSet;

/// Legacy-only calls retain their original bytes and accounting identities.
pub fn compile_server_with_paths(
    nodes: &[Node],
    legacy_relays: &[Relay],
    paths: &[OrderedPath],
    internal_accepts: &[ManagedAcceptance],
    probe_control: Option<&ProbeControl>,
) -> Result<String, CompileError> {
    if paths.is_empty() && internal_accepts.is_empty() && probe_control.is_none() {
        return crate::compile_server_with_relays(nodes, legacy_relays);
    }
    super::context::validate(nodes, legacy_relays, paths, internal_accepts, probe_control)?;
    let original = crate::compile_server_with_relays(nodes, legacy_relays).map_err(|_| {
        CompileError::InvalidPath {
            chain_id: paths
                .first()
                .map(|path| path.chain_id)
                .or_else(|| internal_accepts.first().map(|accept| accept.chain_id))
                .unwrap_or(0),
            position: None,
            reason: "local node or legacy relay configuration is invalid",
        }
    })?;
    let mut config: Value = serde_json::from_str(&original)?;
    let mut paths: Vec<_> = paths.iter().collect();
    paths.sort_by_key(|path| (path.chain_id, path.generation));
    let mut accepts: Vec<_> = internal_accepts.iter().collect();
    accepts.sort_by_key(|accept| (accept.chain_id, accept.generation, accept.position));
    let mut rules = Vec::new();
    for accept in accepts {
        add_acceptance(&mut config, accept);
        // Transit identities have no public-node route and never enter stats.
        rules.push(json!({"inbound":[format!("node-{}",accept.endpoint.node.id)],"auth_user":[acceptance_name(accept)],"action":"route","outbound":"direct"}));
    }
    let active_entries: BTreeSet<_> = paths
        .iter()
        .filter(|path| path.active)
        .map(|path| path.entry_node_id)
        .chain(
            legacy_relays
                .iter()
                .filter(|relay| nodes.iter().any(|node| node.id == relay.entry_node_id))
                .map(|relay| relay.entry_node_id),
        )
        .collect();
    let mut blocked_entries = BTreeSet::new();
    for path in &paths {
        for (index, hop) in path.hops.iter().enumerate() {
            let mut outbound = match hop {
                PathHop::Managed {
                    endpoint,
                    relay_uuid,
                } => crate::protocols::client(
                    &endpoint.node,
                    &Access {
                        user_id: 1,
                        uuid: *relay_uuid,
                        credential: String::new(),
                    },
                ),
                PathHop::External { outbound, .. } => serde_json::to_value(outbound)?,
            };
            outbound["tag"] = json!(super::path_outbound_tag(
                path.chain_id,
                path.generation,
                index + 1
            ));
            if index > 0 {
                outbound["detour"] = json!(super::path_outbound_tag(
                    path.chain_id,
                    path.generation,
                    index
                ));
            }
            // The local bootstrap resolver has no outbound dependency, so DNS
            // cannot recurse through the path it is trying to establish.
            outbound["domain_resolver"] = json!({"server":"chain-bootstrap"});
            config["outbounds"]
                .as_array_mut()
                .expect("compiled outbounds")
                .push(outbound);
        }
        let inbound = format!("node-{}", path.entry_node_id);
        if path.active {
            let capabilities = super::path_capabilities(path)?;
            let blocked = if !capabilities.tcp {
                Some("tcp")
            } else if !capabilities.udp {
                Some("udp")
            } else {
                None
            };
            if let Some(network) = blocked {
                rules.push(json!({"inbound":[inbound],"network":[network],"action":"reject"}));
            }
            rules.push(json!({"inbound":[inbound],"action":"route","outbound":path.final_tag()}));
        } else if !active_entries.contains(&path.entry_node_id)
            && blocked_entries.insert(path.entry_node_id)
        {
            // A candidate can be probed by explicit selector without allowing
            // its public listener to fall through to the direct route.
            rules.push(json!({"inbound":[inbound],"action":"reject"}));
        }
    }
    if let Some(legacy_rules) = config["route"]["rules"].as_array() {
        rules.extend(legacy_rules.iter().cloned());
    }
    if !rules.is_empty() {
        config["route"]["rules"] = Value::Array(rules);
    }
    if !paths.is_empty() {
        config["dns"] =
            json!({"servers":[{"type":"local","tag":"chain-bootstrap"}],"final":"chain-bootstrap"});
    }
    if let Some(probe) = probe_control {
        config["experimental"]["clash_api"] = json!({"external_controller":format!("127.0.0.1:{}",probe.listen_port),"secret":probe.secret});
    }
    crate::pretty(config)
}

fn add_acceptance(config: &mut Value, accept: &ManagedAcceptance) {
    let node = &accept.endpoint.node;
    let tag = format!("node-{}", node.id);
    let identity =
        json!({"name":acceptance_name(accept),"uuid":accept.relay_uuid,"flow":"xtls-rprx-vision"});
    let inbounds = config["inbounds"]
        .as_array_mut()
        .expect("compiled inbounds");
    if let Some(inbound) = inbounds.iter_mut().find(|inbound| inbound["tag"] == tag) {
        inbound["users"]
            .as_array_mut()
            .expect("compiled identities")
            .push(identity);
    } else {
        let mut inbound = crate::protocols::server(node, &[]);
        inbound["users"] = json!([identity]);
        inbounds.push(inbound);
    }
}
