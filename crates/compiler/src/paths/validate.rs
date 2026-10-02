use super::{ManagedEndpointSnapshot, OrderedPath, PathHop};
use crate::{CompileError, Node};
use std::{collections::BTreeSet, net::IpAddr};

pub(super) use super::outbound_validation::external_outbound;

pub(super) fn error(
    path: &OrderedPath,
    position: Option<usize>,
    reason: &'static str,
) -> CompileError {
    CompileError::InvalidPath {
        chain_id: path.chain_id,
        position,
        reason,
    }
}

pub(super) fn endpoint(snapshot: &ManagedEndpointSnapshot) -> Result<(), &'static str> {
    if snapshot.version_id.is_nil() || snapshot.server_id <= 0 {
        return Err("managed endpoint requires a frozen version and server identity");
    }
    if !snapshot.node.users.is_empty() {
        return Err("managed endpoint snapshot must not contain terminal-user grants");
    }
    if !snapshot.node.enabled || !snapshot.node.protocol_config.is_reality() {
        return Err("managed hop must be an enabled Reality endpoint");
    }
    crate::validate_node(&snapshot.node).map_err(|_| "managed endpoint configuration is invalid")
}

pub(super) fn address(host: &str, port: u16) -> (String, u16) {
    let host = crate::unbracket_host(host);
    (
        host.parse::<IpAddr>()
            .map(|ip| ip.to_string())
            .unwrap_or_else(|_| host.to_ascii_lowercase()),
        port,
    )
}

// Display names and grants can change without changing the frozen listener.
pub(super) fn compatible(left: &Node, right: &Node) -> Result<bool, CompileError> {
    let mut left = serde_json::to_value(left)?;
    let mut right = serde_json::to_value(right)?;
    for value in [&mut left, &mut right] {
        let object = value.as_object_mut().expect("serialized node");
        object.remove("users");
        object.remove("name");
    }
    Ok(left == right)
}

/// Validate immutable path inputs without reading mutable panel or runtime state.
pub fn validate_path(path: &OrderedPath) -> Result<(), CompileError> {
    if path.chain_id <= 0
        || path.generation == 0
        || path.entry_node_id <= 0
        || path.entry_server_id <= 0
    {
        return Err(error(
            path,
            None,
            "path requires positive identities and a generation",
        ));
    }
    if path.hops.is_empty() || path.hops.len() > 8 {
        return Err(error(path, None, "path requires one to eight ordered hops"));
    }
    let mut servers = BTreeSet::from([path.entry_server_id]);
    let mut nodes = BTreeSet::from([path.entry_node_id]);
    let mut addresses = BTreeSet::new();
    let mut sources = BTreeSet::new();
    let mut credentials = BTreeSet::new();
    for (index, hop) in path.hops.iter().enumerate() {
        let position = Some(index + 1);
        let host_port = match hop {
            PathHop::Managed {
                endpoint: snapshot,
                relay_uuid,
            } => {
                endpoint(snapshot).map_err(|reason| error(path, position, reason))?;
                if relay_uuid.is_nil() || !credentials.insert(*relay_uuid) {
                    return Err(error(
                        path,
                        position,
                        "managed relay identity is missing or repeated",
                    ));
                }
                if !servers.insert(snapshot.server_id) || !nodes.insert(snapshot.node.id) {
                    return Err(error(
                        path,
                        position,
                        "managed server or endpoint repeats an earlier path position",
                    ));
                }
                address(&snapshot.node.public_host, snapshot.node.public_port())
            }
            PathHop::External {
                source_id,
                identity_epoch,
                node_id,
                version_id,
                source_revision_id,
                outbound,
            } => {
                if *source_id <= 0
                    || *identity_epoch <= 0
                    || node_id.is_nil()
                    || version_id.is_nil()
                    || source_revision_id.is_nil()
                {
                    return Err(error(
                        path,
                        position,
                        "subscription hop requires its complete frozen identity vector",
                    ));
                }
                if !sources.insert((*source_id, *identity_epoch, *node_id)) {
                    return Err(error(
                        path,
                        position,
                        "subscription identity repeats an earlier path position",
                    ));
                }
                external_outbound(outbound).map_err(|reason| error(path, position, reason))?;
                address(&outbound.common().server, outbound.common().server_port)
            }
        };
        if !addresses.insert(host_port) {
            return Err(error(
                path,
                position,
                "endpoint address repeats an earlier path position",
            ));
        }
    }
    let _ = super::network::computed_capabilities(path)?;
    Ok(())
}
