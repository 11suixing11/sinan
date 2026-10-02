use super::{bad, model};
use crate::{
    AppState,
    auth::require_admin,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AdoptNode {
    adopted: bool,
    settings_revision: i64,
    identity_epoch: i64,
    node_version_id: i64,
}

pub(super) async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((source_id, node_id)): Path<(i64, i64)>,
    request: Result<Json<AdoptNode>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<model::NodePreview>> {
    require_admin(&state, &headers).await?;
    let Json(request) = request.map_err(|_| bad("采用请求字段不符合要求"))?;
    if request.settings_revision <= 0 || request.identity_epoch <= 0 || request.node_version_id <= 0
    {
        return Err(bad("采用请求的来源修订号、身份或节点版本无效"));
    }
    let mut tx = state.pool.begin().await?;
    super::super::entitlements::lock(&mut tx).await?;
    let row: Option<(i64,i64,Option<i64>)> = sqlx::query_as("SELECT s.settings_revision,n.identity_epoch,n.current_version_id FROM singbox_external_nodes n JOIN singbox_subscription_sources s ON s.id=n.source_id WHERE n.id=$1 AND n.source_id=$2 AND s.deleted_at IS NULL FOR UPDATE OF n,s")
        .bind(node_id).bind(source_id).fetch_optional(&mut *tx).await?;
    let (settings_revision, identity_epoch, version) = row.ok_or(ApiError::NotFound)?;
    if settings_revision != request.settings_revision
        || identity_epoch != request.identity_epoch
        || version != Some(request.node_version_id)
    {
        return Err(ApiError::Conflict(
            "来源设置或节点版本已变化，请刷新后重新确认采用状态".into(),
        ));
    }
    if request.adopted {
        model::load_version_on(
            &mut tx,
            source_id,
            node_id,
            version.ok_or_else(|| bad("此节点尚无支持的版本"))?,
            true,
        )
        .await?;
        sqlx::query("UPDATE singbox_node_metadata SET deleted_at=NULL,revision=revision+1 WHERE kind='external' AND id=$1 AND deleted_at IS NOT NULL")
            .bind(node_id).execute(&mut *tx).await?;
    } else {
        super::super::catalog::ensure_external_removable(&mut tx, node_id).await?;
    }
    sqlx::query("UPDATE singbox_external_nodes SET adopted=$2 WHERE id=$1")
        .bind(node_id)
        .bind(request.adopted)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    let node = model::nodes_on(&state.pool, source_id)
        .await?
        .into_iter()
        .find(|node| node.id == Some(node_id))
        .ok_or(ApiError::NotFound)?;
    Ok(Json(node))
}
