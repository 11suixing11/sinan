mod commands;
mod probes;
mod state;

use crate::{SharedState, artifacts::PanelClient};
use anyhow::Result;
use sinan_adapter_sdk::Privileged;
use std::sync::Arc;
use tokio::sync::watch;

pub async fn run(
    allow_remote_commands: bool,
    state: SharedState,
    ops: Arc<dyn Privileged>,
    clients: watch::Receiver<Option<Arc<PanelClient>>>,
    retirement: Arc<crate::retirement::Retirement>,
) -> Result<()> {
    tokio::try_join!(
        commands::run(
            allow_remote_commands,
            state.clone(),
            ops.clone(),
            clients.clone(),
            retirement.clone()
        ),
        probes::run(state, ops, clients, retirement)
    )?;
    Ok(())
}
