use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::Serialize;
use serde_json::Value;
use sinan_protocol::now_timestamp;
use sqlx::{FromRow, Postgres, Transaction};

// A current device declaration, an explicit administrator choice or preserved
// legacy configuration is evidence of enablement. Unknown capabilities are not.
pub(super) const SOURCE_SQL: &str = "CASE WHEN s.capabilities ? 'singbox' THEN 'agent_capability' WHEN p.enabled AND p.source <> 'agent_capability' THEN p.source WHEN EXISTS(SELECT 1 FROM nodes n WHERE n.server_id=s.id) THEN 'legacy_nodes' WHEN EXISTS(SELECT 1 FROM deployments d WHERE d.server_id=s.id AND d.module='singbox') THEN 'legacy_deployments' END";

#[derive(Serialize, FromRow)]
pub struct PluginServer {
    pub id: i64,
    pub name: String,
    #[serde(skip)]
    pub last_seen: Option<i64>,
    #[serde(skip)]
    pub capabilities: Value,
    pub source: Option<String>,
    #[sqlx(default)]
    pub enabled: bool,
    #[sqlx(default)]
    pub online: bool,
    #[sqlx(default)]
    pub agent_supported: bool,
    #[sqlx(default)]
    pub read_only: bool,
}
impl PluginServer {
    fn view(mut self) -> Self {
        self.enabled = self.source.is_some();
        self.agent_supported = self
            .capabilities
            .as_array()
            .is_some_and(|caps| caps.iter().any(|cap| cap == "singbox"));
        self.online = self
            .last_seen
            .is_some_and(|seen| now_timestamp().saturating_sub(seen) <= 60);
        self.read_only = self.enabled && self.source.as_deref() != Some("administrator");
        self
    }
}
fn query() -> String {
    format!(
        "SELECT s.id,s.name,s.last_seen,s.capabilities,{SOURCE_SQL} AS source FROM servers s LEFT JOIN server_plugins p ON p.server_id=s.id AND p.plugin='sing-box' WHERE s.deleted_at IS NULL"
    )
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<PluginServer>>> {
    auth::require_admin(&state, &headers).await?;
    let rows = sqlx::query_as::<_, PluginServer>(&format!("{} ORDER BY s.id", query()))
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows.into_iter().map(PluginServer::view).collect()))
}
pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<Json<PluginServer>> {
    auth::require_admin(&state, &headers).await?;
    let row = sqlx::query_as::<_, PluginServer>(&format!("{} AND s.id=$1", query()))
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row.view()))
}
pub async fn enable(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
) -> ApiResult<(StatusCode, Json<PluginServer>)> {
    auth::require_admin(&state, &headers).await?;
    let mut tx = state.pool.begin().await?;
    super::business::lock_server(&mut tx, id).await?;
    sqlx::query("INSERT INTO server_plugins(server_id,plugin,source,enabled_at) VALUES($1,'sing-box','administrator',$2) ON CONFLICT(server_id,plugin) DO UPDATE SET enabled=TRUE,source=CASE WHEN server_plugins.source='agent_capability' THEN 'administrator' ELSE server_plugins.source END")
        .bind(id).bind(now_timestamp()).execute(&mut *tx).await?;
    tx.commit().await?;
    let row = get(State(state), headers, Path(id)).await?.0;
    Ok((StatusCode::OK, Json(row)))
}

pub(super) async fn require_enabled(tx: &mut Transaction<'_, Postgres>, id: i64) -> ApiResult<()> {
    let source: Option<String> = sqlx::query_scalar(&format!("SELECT {SOURCE_SQL} FROM servers s LEFT JOIN server_plugins p ON p.server_id=s.id AND p.plugin='sing-box' WHERE s.id=$1 AND s.deleted_at IS NULL"))
        .bind(id).fetch_one(&mut **tx).await?;
    if source.is_none() {
        return Err(ApiError::Conflict(
            "此服务器尚未启用 sing-box，请先在系统的插件设置中明确启用".into(),
        ));
    }
    Ok(())
}
