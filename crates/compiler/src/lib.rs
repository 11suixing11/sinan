#![forbid(unsafe_code)]

use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, net::IpAddr};
use thiserror::Error;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Access {
    pub user_id: i64,
    pub uuid: Uuid,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: i64,
    pub name: String,
    pub port: u16,
    pub public_host: String,
    pub sni: String,
    pub private_key: String,
    pub public_key: String,
    pub short_id: String,
    pub users: Vec<Access>,
}

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("node {node_id}: {reason}")]
    InvalidNode { node_id: i64, reason: String },
    #[error("duplicate node id {0}")]
    DuplicateNode(i64),
    #[error("duplicate port {0} on the same server")]
    DuplicatePort(u16),
    #[error("duplicate user {user_id} on node {node_id}")]
    DuplicateUser { node_id: i64, user_id: i64 },
    #[error("user id must be positive: {0}")]
    InvalidUserId(i64),
    #[error("user {0} has no authorized nodes")]
    NoAuthorizedNodes(i64),
    #[error("cannot serialize configuration: {0}")]
    Json(#[from] serde_json::Error),
}

pub fn stat_name(user_id: i64, node_id: i64) -> String {
    format!("u{user_id}_n{node_id}")
}

/// Compiles one server's full desired configuration into deterministic bytes.
pub fn compile_server(nodes: &[Node]) -> Result<String, CompileError> {
    let nodes = sorted_validated(nodes, true)?;
    let mut inbounds = Vec::new();
    let mut stats_users = BTreeSet::new();
    for node in nodes {
        if node.users.is_empty() {
            continue;
        }
        let mut users: Vec<_> = node.users.iter().collect();
        users.sort_by_key(|user| user.user_id);
        let users: Vec<_> = users
            .into_iter()
            .map(|user| {
                let name = stat_name(user.user_id, node.id);
                stats_users.insert(name.clone());
                json!({"name": name, "uuid": user.uuid, "flow": "xtls-rprx-vision"})
            })
            .collect();
        inbounds.push(json!({
            "type": "vless", "tag": format!("node-{}", node.id), "listen": "::", "listen_port": node.port,
            "users": users,
            "tls": { "enabled": true, "server_name": node.sni, "reality": {
                "enabled": true, "handshake": { "server": node.sni, "server_port": 443 },
                "private_key": node.private_key, "short_id": [node.short_id]
            } }
        }));
    }
    pretty(json!({
        "log": { "level": "warn", "timestamp": true },
        "inbounds": inbounds,
        "outbounds": [{ "type": "direct", "tag": "direct" }],
        "route": { "final": "direct" },
        "experimental": { "v2ray_api": { "listen": "127.0.0.1:18085", "stats": {
            "enabled": true, "users": stats_users
        } } }
    }))
}

/// Callers supply only successfully applied nodes; only this user's credentials are emitted.
pub fn compile_client(nodes: &[Node], user_id: i64) -> Result<String, CompileError> {
    let accesses = authorized_nodes(nodes, user_id)?;
    if accesses.is_empty() {
        return Err(CompileError::NoAuthorizedNodes(user_id));
    }
    let tags: Vec<_> = accesses
        .iter()
        .map(|(node, _)| format!("node-{}", node.id))
        .collect();
    let mut outbounds = vec![json!({ "type": "selector", "tag": "proxy", "outbounds": tags })];
    for (node, access) in accesses {
        outbounds.push(json!({
            "type": "vless", "tag": format!("node-{}", node.id),
            "server": unbracket_host(&node.public_host), "server_port": node.port,
            "uuid": access.uuid, "flow": "xtls-rprx-vision",
            "tls": {
                "enabled": true, "server_name": node.sni,
                "utls": { "enabled": true, "fingerprint": "chrome" },
                "reality": { "enabled": true, "public_key": node.public_key, "short_id": node.short_id }
            }
        }));
    }
    outbounds.push(json!({ "type": "direct", "tag": "direct" }));
    pretty(json!({
        "log": { "level": "warn", "timestamp": true },
        "inbounds": [{ "type": "mixed", "tag": "mixed-in", "listen": "127.0.0.1", "listen_port": 2080 }],
        "outbounds": outbounds,
        "route": { "final": "proxy" }
    }))
}

pub fn subscription_links(nodes: &[Node], user_id: i64) -> Result<String, CompileError> {
    let accesses = authorized_nodes(nodes, user_id)?;
    let links: Vec<_> = accesses.into_iter().map(|(node, access)| {
        let host = unbracket_host(&node.public_host);
        let host = if host.contains(':') { format!("[{host}]") } else { host.to_string() };
        format!(
            "vless://{}@{}:{}?encryption=none&flow=xtls-rprx-vision&security=reality&sni={}&fp=chrome&pbk={}&sid={}&type=tcp#{}",
            access.uuid, host, node.port, percent_encode(&node.sni), percent_encode(&node.public_key),
            percent_encode(&node.short_id), percent_encode(&node.name)
        )
    }).collect();
    Ok(STANDARD.encode(links.join("\n")))
}

fn pretty(value: Value) -> Result<String, CompileError> {
    Ok(format!("{}\n", serde_json::to_string_pretty(&value)?))
}

fn sorted_validated(nodes: &[Node], same_server: bool) -> Result<Vec<&Node>, CompileError> {
    let mut nodes: Vec<_> = nodes.iter().collect();
    nodes.sort_by_key(|node| node.id);
    let mut ids = BTreeSet::new();
    let mut ports = BTreeSet::new();
    for node in &nodes {
        if !ids.insert(node.id) {
            return Err(CompileError::DuplicateNode(node.id));
        }
        validate_node(node)?;
        if same_server && !ports.insert(node.port) {
            return Err(CompileError::DuplicatePort(node.port));
        }
        if same_server && node.port == 18085 {
            return Err(invalid_node(
                node,
                "port conflicts with the local statistics API",
            ));
        }
    }
    Ok(nodes)
}

fn authorized_nodes(nodes: &[Node], user_id: i64) -> Result<Vec<(&Node, &Access)>, CompileError> {
    if user_id <= 0 {
        return Err(CompileError::InvalidUserId(user_id));
    }
    let accesses: Vec<_> = sorted_validated(nodes, false)?
        .into_iter()
        .filter_map(|node| {
            node.users
                .iter()
                .find(|access| access.user_id == user_id)
                .map(|access| (node, access))
        })
        .collect();
    Ok(accesses)
}

fn validate_node(node: &Node) -> Result<(), CompileError> {
    if node.id <= 0 {
        return Err(invalid_node(node, "id must be positive"));
    }
    if node.name.trim().is_empty() || node.name.chars().any(char::is_control) {
        return Err(invalid_node(
            node,
            "name must be nonempty and contain no control characters",
        ));
    }
    if node.port == 0 {
        return Err(invalid_node(node, "port must be nonzero"));
    }
    if !valid_public_host(&node.public_host) {
        return Err(invalid_node(
            node,
            "public_host must be a DNS name or IP address without a port",
        ));
    }
    if !valid_dns_name(&node.sni) || node.sni.parse::<IpAddr>().is_ok() {
        return Err(invalid_node(node, "sni must be a DNS hostname"));
    }
    if !valid_key(&node.private_key) || !valid_key(&node.public_key) {
        return Err(invalid_node(
            node,
            "Reality keys must encode 32 bytes as unpadded URL-safe base64",
        ));
    }
    if node.short_id.len() != 8 || !node.short_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(invalid_node(
            node,
            "short_id must contain exactly eight hexadecimal digits",
        ));
    }
    let mut users = BTreeSet::new();
    for access in &node.users {
        if access.user_id <= 0 {
            return Err(CompileError::InvalidUserId(access.user_id));
        }
        if !users.insert(access.user_id) {
            return Err(CompileError::DuplicateUser {
                node_id: node.id,
                user_id: access.user_id,
            });
        }
    }
    Ok(())
}

fn invalid_node(node: &Node, reason: &str) -> CompileError {
    CompileError::InvalidNode {
        node_id: node.id,
        reason: reason.into(),
    }
}

fn valid_key(value: &str) -> bool {
    URL_SAFE_NO_PAD
        .decode(value)
        .is_ok_and(|bytes| bytes.len() == 32 && URL_SAFE_NO_PAD.encode(bytes) == value)
}

fn valid_public_host(value: &str) -> bool {
    let host = unbracket_host(value);
    if host != value {
        return host.parse::<std::net::Ipv6Addr>().is_ok();
    }
    host.parse::<IpAddr>().is_ok() || valid_dns_name(host)
}

fn unbracket_host(value: &str) -> &str {
    value
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
        .unwrap_or(value)
}

fn valid_dns_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 253
        && value.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn percent_encode(value: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(char::from(HEX[usize::from(byte >> 4)]));
            encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
    }
    encoded
}
