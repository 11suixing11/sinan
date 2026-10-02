use crate::{Node, external::NormalizedOutbound};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Immutable endpoint snapshots contain no terminal-user grants.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedEndpointSnapshot {
    pub version_id: Uuid,
    pub server_id: i64,
    pub node: Node,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PathHop {
    Managed {
        endpoint: Box<ManagedEndpointSnapshot>,
        relay_uuid: Uuid,
    },
    External {
        source_id: i64,
        identity_epoch: i64,
        node_id: Uuid,
        version_id: Uuid,
        source_revision_id: Uuid,
        outbound: Box<NormalizedOutbound>,
    },
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrderedPath {
    pub chain_id: i64,
    pub generation: u64,
    pub entry_node_id: i64,
    pub entry_server_id: i64,
    pub hops: Vec<PathHop>,
    pub active: bool,
}

impl OrderedPath {
    pub fn final_tag(&self) -> String {
        path_outbound_tag(self.chain_id, self.generation, self.hops.len())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagedAcceptance {
    pub endpoint: ManagedEndpointSnapshot,
    pub chain_id: i64,
    pub generation: u64,
    /// One-based position after the public entry.
    pub position: u8,
    pub relay_uuid: Uuid,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeControl {
    pub listen_port: u16,
    pub secret: String,
}

pub fn path_outbound_tag(chain_id: i64, generation: u64, position: usize) -> String {
    format!("chain-{chain_id}-g{generation}-h{position}")
}

pub(super) fn acceptance_name(accept: &ManagedAcceptance) -> String {
    format!(
        "relay_{}_g{}_h{}",
        accept.chain_id, accept.generation, accept.position
    )
}
