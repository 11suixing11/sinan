use super::{CompileError, Node, compile_server, pretty, protocols, unbracket_host};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use uuid::Uuid;

/// A dedicated two-hop route. Only the panel and the two runtimes receive this secret.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Relay {
    #[serde(default)]
    pub fingerprint: crate::Fingerprint,
    pub chain_id: i64,
    pub entry_node_id: i64,
    pub exit_node_id: i64,
    pub uuid: Uuid,
    pub public_host: String,
    pub port: u16,
    pub sni: String,
    pub public_key: String,
    pub short_id: String,
}

pub fn compile_server_with_relays(
    nodes: &[Node],
    relays: &[Relay],
) -> Result<String, CompileError> {
    let original = compile_server(nodes)?;
    if relays.is_empty() {
        return Ok(original);
    }
    let mut config: Value = serde_json::from_str(&original)?;
    let mut relays: Vec<_> = relays.iter().collect();
    relays.sort_by_key(|relay| relay.chain_id);
    let mut ids = BTreeSet::new();
    let mut entries = BTreeSet::new();
    let mut rules = Vec::new();
    for relay in relays {
        let entry = nodes.iter().find(|node| node.id == relay.entry_node_id);
        let exit = nodes.iter().find(|node| node.id == relay.exit_node_id);
        if relay.chain_id <= 0
            || relay.entry_node_id <= 0
            || relay.exit_node_id <= 0
            || relay.entry_node_id == relay.exit_node_id
            || !ids.insert(relay.chain_id)
            || !entries.insert(relay.entry_node_id)
            || (entry.is_some() == exit.is_some())
            || entry.is_some_and(|node| !node.enabled || !node.protocol_config.is_reality())
            || exit.is_some_and(|node| !node.enabled || !node.protocol_config.is_reality())
            || relay.port == 0
            || !super::valid_public_host(&relay.public_host)
            || !super::valid_dns_name(&relay.sni)
            || relay.sni.parse::<std::net::IpAddr>().is_ok()
            || !super::valid_key(&relay.public_key)
            || relay.short_id.len() != 8
            || !relay.short_id.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(CompileError::InvalidNode {
                node_id: relay.entry_node_id,
                reason: "invalid, duplicate, or non-two-host VLESS Reality relay".into(),
            });
        }
        let tag = format!("chain-{}", relay.chain_id);
        if entry.is_some_and(|node| !node.users.is_empty()) {
            config["outbounds"].as_array_mut().expect("compiled outbounds").push(json!({
                "type":"vless", "tag":tag, "server":unbracket_host(&relay.public_host),
                "server_port":relay.port, "uuid":relay.uuid, "flow":"xtls-rprx-vision",
                "tls":{"enabled":true,"server_name":relay.sni,"utls":{"enabled":true,"fingerprint":relay.fingerprint},
                    "reality":{"enabled":true,"public_key":relay.public_key,"short_id":relay.short_id}}
            }));
            rules.push(json!({"inbound":[format!("node-{}",relay.entry_node_id)],"action":"route","outbound":tag}));
        }
        if let Some(exit) = exit {
            let tag = format!("node-{}", exit.id);
            let inbounds = config["inbounds"]
                .as_array_mut()
                .expect("compiled inbounds");
            let relay_identity = json!({"name":format!("relay_{}",relay.chain_id),"uuid":relay.uuid,"flow":"xtls-rprx-vision"});
            if let Some(existing) = inbounds.iter_mut().find(|value| value["tag"] == tag) {
                existing["users"]
                    .as_array_mut()
                    .expect("compiled identities")
                    .push(relay_identity);
            } else {
                let mut inbound = protocols::server(exit, &[]);
                inbound["users"] = json!([relay_identity]);
                inbounds.push(inbound);
            }
            // Relay identities intentionally never enter stats.users: only the entry is billed.
        }
    }
    config["route"]["rules"] = Value::Array(rules);
    pretty(config)
}
