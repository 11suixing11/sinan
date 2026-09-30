pub mod singbox;

use crate::AppState;
use axum::Router;

pub fn router() -> Router<AppState> {
    singbox::router()
}

pub async fn run(state: AppState) {
    singbox::publisher::run(state).await
}

pub async fn ingest_usage(
    state: &AppState,
    server_id: i64,
    batch: sinan_protocol::UsageBatch,
) -> anyhow::Result<()> {
    singbox::usage::ingest(state, server_id, batch).await
}

pub async fn manifest_modules(
    state: &AppState,
    server_id: i64,
    info: &serde_json::Value,
) -> crate::error::ApiResult<std::collections::BTreeMap<String, sinan_protocol::ModuleManifest>> {
    let mut modules = std::collections::BTreeMap::new();
    if let Some(module) = singbox::agent::manifest_module(state, server_id, info).await? {
        modules.insert("singbox".into(), module);
    }
    Ok(modules)
}

pub async fn bundle(state: &AppState, server_id: i64, rev: i64) -> crate::error::ApiResult<String> {
    singbox::agent::bundle(state, server_id, rev).await
}
