use crate::{
    AppState, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::{Value, json};
use sinan_compiler::Node;
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct SubscriptionQuery {
    pub format: Option<String>,
    pub download: Option<bool>,
}

fn validate_format(format: &str) -> ApiResult<()> {
    if !matches!(format, "links" | "singbox") {
        return Err(ApiError::BadRequest(
            "订阅格式仅支持 links 或 singbox".into(),
        ));
    }
    Ok(())
}

struct Snapshot {
    nodes: Vec<Node>,
    granted_nodes: i64,
    eligible_nodes: usize,
    entitlement: super::entitlements::Entitlement,
}

impl Snapshot {
    fn message(&self) -> (&'static str, String) {
        if !self.entitlement.allowed {
            let reason = match self.entitlement.status.as_str() {
                "expired" => "套餐已到期，请先续期或重新分配套餐",
                "exhausted" => "当前套餐周期流量已用尽，请等待重置或调整套餐",
                "not_started" => "套餐尚未生效，请等待开始时间",
                _ => "当前套餐状态不允许使用代理",
            };
            return ("blocked", reason.into());
        }
        if self.nodes.is_empty() {
            return (
                "empty",
                if self.granted_nodes == 0 {
                    "尚未分配节点，请先授权节点或分配策略组"
                } else {
                    "暂无可订阅节点，请检查节点启用状态、链路两端和配置部署结果"
                }
                .into(),
            );
        }
        (
            "ready",
            format!(
                "{} 个节点可生成订阅；仅包含当前有效且已成功应用的授权",
                self.nodes.len()
            ),
        )
    }

    fn content(&self, user_id: i64, format: &str) -> ApiResult<String> {
        if format == "links" {
            sinan_compiler::subscription_links(&self.nodes, user_id).map_err(|error| match error {
                sinan_compiler::CompileError::RequiresJson { .. } => ApiError::Conflict(
                    "此订阅包含需要完整配置的协议，请使用 format=singbox 下载 sing-box JSON".into(),
                ),
                other => ApiError::Internal(anyhow::Error::from(other)),
            })
        } else {
            if self.nodes.is_empty() {
                return Err(ApiError::Conflict(self.message().1));
            }
            Ok(
                sinan_compiler::compile_client(&self.nodes, user_id)
                    .map_err(anyhow::Error::from)?,
            )
        }
    }
}

async fn load(tx: &mut Transaction<'_, Postgres>, user_id: i64) -> ApiResult<Snapshot> {
    let at = sinan_protocol::now_timestamp();
    let entitlement = sqlx::query_as("SELECT * FROM singbox_entitlements($1) WHERE user_id=$2")
        .bind(at)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(ApiError::NotFound)?;
    let granted_nodes =
        sqlx::query_scalar("SELECT COUNT(*) FROM singbox_desired_accesses WHERE user_id=$1")
            .bind(user_id)
            .fetch_one(&mut **tx)
            .await?;
    let accesses = sqlx::query("SELECT a.node_id,a.uuid,a.credential FROM singbox_eligible_accesses($2) a WHERE a.user_id=$1")
        .bind(user_id).bind(at).fetch_all(&mut **tx).await?;
    let mut current = BTreeMap::<i64, (Uuid, String)>::new();
    for row in accesses {
        let node_id: i64 = row.get("node_id");
        let chain: Option<i64> = sqlx::query_scalar(
            "SELECT id FROM singbox_chains WHERE entry_node_id=$1 AND deleted_at IS NULL",
        )
        .bind(node_id)
        .fetch_optional(&mut **tx)
        .await?;
        if let Some(chain) = chain
            && !super::ordered_paths::lifecycle::qualified(tx, chain).await?
        {
            continue;
        }
        current.insert(node_id, (row.get("uuid"), row.get("credential")));
    }
    let eligible_nodes = current.len();
    let snapshots: Vec<(i64,i64,serde_json::Value)> = sqlx::query_as("SELECT d.server_id,d.rev,d.source_json FROM deployments d JOIN server_module_status m ON m.server_id=d.server_id AND m.module=d.module AND m.applied_rev=d.rev JOIN servers s ON s.id=d.server_id WHERE m.module='singbox' AND m.healthy AND s.deleted_at IS NULL AND EXISTS (SELECT 1 FROM nodes n WHERE n.server_id=s.id AND n.id=ANY($1)) ORDER BY d.server_id")
        .bind(current.keys().copied().collect::<Vec<_>>()).fetch_all(&mut **tx).await?;
    let mut nodes = Vec::new();
    for (server, revision, snapshot) in snapshots {
        let snapshot =
            super::ordered_paths::models::public_nodes(snapshot).map_err(anyhow::Error::from)?;
        let projection:BTreeMap<i64,serde_json::Value>=sqlx::query_as::<_,(i64,serde_json::Value)>("SELECT node_id,public_fields FROM singbox_deployment_public_projection WHERE server_id=$1 AND module='singbox' AND revision=$2").bind(server).bind(revision).fetch_all(&mut **tx).await?.into_iter().collect();
        for mut node in snapshot {
            if let Some(display) = projection.get(&node.id) {
                if let Some(name) = display.get("name").and_then(serde_json::Value::as_str) {
                    node.name = name.into();
                }
                if let Some(host) = display
                    .get("public_host")
                    .and_then(serde_json::Value::as_str)
                {
                    node.public_host = host.into();
                }
                if let Some(sni) = display.get("sni").and_then(serde_json::Value::as_str) {
                    node.sni = sni.into();
                }
                if let Some(port) = display
                    .get("public_port")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| u16::try_from(value).ok())
                    .filter(|port| *port > 0)
                {
                    node.settings.public_port = Some(port);
                }
            }
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
    nodes.retain(|node| node.enabled);
    nodes.sort_by_key(|node| node.id);
    Ok(Snapshot {
        nodes,
        granted_nodes,
        eligible_nodes,
        entitlement,
    })
}

fn content_type(format: &str) -> &'static str {
    if format == "singbox" {
        "application/json; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    }
}
fn filename(user_id: i64, format: &str) -> String {
    format!(
        "sinan-{user_id}.{}",
        if format == "singbox" { "json" } else { "txt" }
    )
}

pub async fn get(
    State(state): State<AppState>,
    Path(token): Path<String>,
    Query(query): Query<SubscriptionQuery>,
) -> ApiResult<Response> {
    // Existing token-only URLs retain their original Reality links format.
    let format = query.format.as_deref().unwrap_or("links");
    validate_format(format)?;
    if token.is_empty() || token.len() > 512 {
        return Err(ApiError::NotFound);
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout='5s'")
        .execute(&mut *tx)
        .await?;
    let user_id = sqlx::query_scalar(
        "SELECT id FROM users WHERE subscription_token=$1 AND deleted_at IS NULL",
    )
    .bind(token)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    let snapshot = load(&mut tx, user_id).await?;
    tx.commit().await?;
    let body = snapshot.content(user_id, format)?;
    let mut response = (
        [
            (header::CONTENT_TYPE, content_type(format)),
            (header::CACHE_CONTROL, "no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            (header::REFERRER_POLICY, "no-referrer"),
        ],
        body,
    )
        .into_response();
    if query.download == Some(true) {
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename(user_id, format))
                .parse()
                .map_err(anyhow::Error::from)?,
        );
    }
    Ok(response)
}

pub async fn preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<i64>,
    Query(query): Query<SubscriptionQuery>,
) -> ApiResult<Response> {
    auth::require_admin(&state, &headers).await?;
    let format = query.format.as_deref().unwrap_or("singbox");
    validate_format(format)?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL statement_timeout='5s'")
        .execute(&mut *tx)
        .await?;
    let token: String = sqlx::query_scalar(
        "SELECT subscription_token FROM users WHERE id=$1 AND deleted_at IS NULL",
    )
    .bind(id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    let snapshot = load(&mut tx, id).await?;
    tx.commit().await?;
    let mut formats = vec!["singbox"];
    if snapshot
        .nodes
        .iter()
        .all(|node| node.protocol_config.is_reality())
    {
        formats.push("links");
    }
    let (mut status, mut message) = snapshot.message();
    let content = if !snapshot.nodes.is_empty() && snapshot.entitlement.allowed {
        match snapshot.content(id, format) {
            Ok(content) => Some(content),
            Err(ApiError::Conflict(reason)) => {
                status = "format_unavailable";
                message = reason;
                None
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    let ready_nodes: Vec<Value> = snapshot
        .nodes
        .iter()
        .map(|node| json!({"id":node.id,"name":node.name,"protocol":node.protocol_config.kind()}))
        .collect();
    Ok(([
        (header::CACHE_CONTROL, "no-store"), (header::REFERRER_POLICY, "no-referrer"),
    ], Json(json!({
        "format":format,"status":status,"message":message,"available_formats":formats,
        "granted_nodes":snapshot.granted_nodes,"eligible_nodes":snapshot.eligible_nodes,"ready_nodes":ready_nodes,
        "content":content,"filename":filename(id,format),"content_type":content_type(format),
        "entitlement":snapshot.entitlement,"subscription_url":format!("{}/sub/{token}",state.config.public_url)
    }))).into_response())
}
