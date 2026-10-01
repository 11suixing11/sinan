use crate::{
    AppState,
    error::{ApiError, ApiResult},
};
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use sinan_compiler::Node;
use sqlx::Row;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SubscriptionQuery {
    pub format: Option<String>,
}

pub async fn get(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<SubscriptionQuery>,
) -> ApiResult<Response> {
    let format = query.format.as_deref().unwrap_or("links");
    if !matches!(format, "links" | "singbox") {
        return Err(ApiError::BadRequest(
            "订阅格式仅支持 links 或 singbox".into(),
        ));
    }
    if token.is_empty() || token.len() > 512 {
        return Err(ApiError::NotFound);
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let user_id: i64 = sqlx::query_scalar(
        "SELECT id FROM users WHERE subscription_token=$1 AND deleted_at IS NULL",
    )
    .bind(token)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    let accesses = sqlx::query("SELECT a.node_id,a.uuid,a.credential FROM singbox_eligible_accesses($2) a WHERE a.user_id=$1 AND NOT EXISTS (SELECT 1 FROM singbox_chains c JOIN nodes n ON n.id=c.entry_node_id JOIN nodes e ON e.id=c.exit_node_id WHERE c.entry_node_id=a.node_id AND (SELECT COUNT(*) FROM server_module_status m JOIN servers s ON s.id=m.server_id WHERE m.server_id=ANY(ARRAY[n.server_id,e.server_id]) AND m.module='singbox' AND m.healthy AND m.applied_rev=m.target_rev AND s.dirty_at IS NULL AND s.deleted_at IS NULL) <> 2)")
        .bind(user_id).bind(sinan_protocol::now_timestamp()).fetch_all(&mut *tx).await?;
    let current: BTreeMap<i64, (Uuid, String)> = accesses
        .into_iter()
        .map(|row| (row.get("node_id"), (row.get("uuid"), row.get("credential"))))
        .collect();
    let snapshots: Vec<serde_json::Value> = sqlx::query_scalar("SELECT d.source_json FROM deployments d JOIN server_module_status m ON m.server_id=d.server_id AND m.module=d.module AND m.applied_rev=d.rev JOIN servers s ON s.id=d.server_id WHERE m.module='singbox' AND m.healthy AND s.deleted_at IS NULL ORDER BY d.server_id")
        .fetch_all(&mut *tx).await?;
    tx.commit().await?;
    let mut nodes = Vec::new();
    for snapshot in snapshots {
        let snapshot: Vec<Node> = serde_json::from_value(snapshot).map_err(anyhow::Error::from)?;
        for mut node in snapshot {
            node.users.retain(|access| {
                access.user_id == user_id
                    && current.get(&node.id).is_some_and(|(uuid, credential)| {
                        *uuid == access.uuid && credential == &access.credential
                    })
            });
            if !node.users.is_empty() {
                nodes.push(node);
            }
        }
    }
    let (content_type, body) = if format == "links" {
        (
            "text/plain; charset=utf-8",
            sinan_compiler::subscription_links(&nodes, user_id).map_err(|error| match error {
                sinan_compiler::CompileError::RequiresJson { .. } => ApiError::Conflict(
                    "此订阅包含需要完整配置的协议，请使用 format=singbox 下载 sing-box JSON".into(),
                ),
                other => ApiError::Internal(anyhow::Error::from(other)),
            })?,
        )
    } else {
        if nodes.is_empty() {
            return Err(ApiError::Conflict(
                "暂无已成功应用且健康的授权节点，请等待部署完成".into(),
            ));
        }
        (
            "application/json; charset=utf-8",
            sinan_compiler::compile_client(&nodes, user_id).map_err(anyhow::Error::from)?,
        )
    };
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response())
}
