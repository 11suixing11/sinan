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
    metadata_revision: i64,
}

pub(super) async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((source_id, node_id)): Path<(i64, i64)>,
    request: Result<Json<AdoptNode>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<model::NodePreview>> {
    require_admin(&state, &headers).await?;
    super::super::source_migration::ensure_numbered_writable(&state.pool).await?;
    let Json(request) = request.map_err(|_| bad("采用请求字段不符合要求"))?;
    if [
        request.settings_revision,
        request.identity_epoch,
        request.node_version_id,
    ]
    .iter()
    .any(|value| !(1..=super::super::business::MAX_SAFE_INTEGER).contains(value))
        || !(0..=super::super::business::MAX_SAFE_INTEGER).contains(&request.metadata_revision)
    {
        return Err(bad("采用请求的来源修订号、身份、节点版本或管理修订号无效"));
    }
    let mut tx = state.pool.begin().await?;
    super::super::entitlements::lock(&mut tx).await?;
    let row: Option<(i64,i64,Option<i64>,bool,i64,bool)> = sqlx::query_as("SELECT s.settings_revision,n.identity_epoch,n.current_version_id,n.adopted,COALESCE(m.revision,0),m.deleted_at IS NOT NULL FROM singbox_external_nodes n JOIN singbox_subscription_sources s ON s.id=n.source_id LEFT JOIN singbox_node_metadata m ON m.kind='external' AND m.id=n.id WHERE n.id=$1 AND n.source_id=$2 AND s.deleted_at IS NULL FOR UPDATE OF n,s")
        .bind(node_id).bind(source_id).fetch_optional(&mut *tx).await?;
    let (settings_revision, identity_epoch, version, adopted, metadata_revision, deleted) =
        row.ok_or(ApiError::NotFound)?;
    if settings_revision != request.settings_revision
        || identity_epoch != request.identity_epoch
        || version != Some(request.node_version_id)
        || metadata_revision != request.metadata_revision
    {
        return Err(ApiError::Conflict(
            "来源、节点版本或管理设置已变化，请刷新后重新确认采用状态".into(),
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
    } else {
        super::super::catalog::ensure_external_removable(&mut tx, node_id).await?;
    }
    // Adoption and catalog deletion share one metadata CAS. A current source
    // version alone cannot revive a locally deleted resource from an old draft.
    if adopted != request.adopted || (request.adopted && deleted) {
        let next_revision = metadata_revision
            .checked_add(1)
            .filter(|next| *next <= super::super::business::MAX_SAFE_INTEGER)
            .ok_or_else(|| ApiError::Conflict("节点管理修订号已达上限".into()))?;
        sqlx::query("INSERT INTO singbox_node_metadata(kind,id,revision) VALUES('external',$1,$2) ON CONFLICT(kind,id) DO UPDATE SET revision=$2,deleted_at=CASE WHEN $3 THEN NULL ELSE singbox_node_metadata.deleted_at END")
            .bind(node_id).bind(next_revision).bind(request.adopted).execute(&mut *tx).await?;
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
