use super::Runtime;
use anyhow::Result;
use serde_json::{Value, json};
use std::{collections::BTreeMap, sync::atomic::Ordering};

pub(super) fn snapshot(runtime: &Runtime) -> Result<Value> {
    let applied = runtime.applied()?;
    let state = runtime
        .state
        .lock()
        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
    let mut healthy = BTreeMap::new();
    for module in runtime.modules.iter() {
        healthy.insert(
            module.clone(),
            state
                .get_json::<bool>(&format!("health:{module}"))?
                .unwrap_or(false),
        );
    }
    Ok(
        json!({"agent_version":env!("CARGO_PKG_VERSION"),"pid":std::process::id(),"connected":runtime.connected.load(Ordering::Relaxed),"applied":applied,"healthy":healthy,"pending_batches":state.pending_usage_count()?,"pending_telemetry":state.pending_telemetry_count()?}),
    )
}
