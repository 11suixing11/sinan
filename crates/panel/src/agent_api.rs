use crate::{
    AgentConnection, AppState, artifacts, auth,
    error::{ApiError, ApiResult},
};
use axum::{
    Json,
    extract::{
        Path, State,
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderMap, header},
    response::{IntoResponse, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ed25519_dalek::{Signature, VerifyingKey};
use futures_util::{SinkExt, StreamExt};
use sinan_protocol::{
    AppliedRevisions, ApplyResult, ApplyStatus, AuthChallenge, Envelope, HelloAck, Manifest,
    ManifestChanged, Message, ModuleManifest, now_timestamp,
};
use sqlx::Row;
use std::{collections::BTreeMap, time::Duration};
use tokio::{
    sync::mpsc,
    time::{Instant, timeout},
};
use uuid::Uuid;

pub async fn websocket(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    ws.max_message_size(1024 * 1024)
        .on_upgrade(move |socket| async move {
            if let Err(error) = connection(socket, state).await {
                tracing::debug!(%error, "device connection ended");
            }
        })
}

async fn connection(mut socket: WebSocket, state: AppState) -> anyhow::Result<()> {
    let nonce = auth::random_token();
    send(
        &mut socket,
        Envelope::new(
            "auth.challenge",
            AuthChallenge {
                nonce: nonce.clone(),
                server_time: now_timestamp(),
            },
        )?,
    )
    .await?;
    let text = timeout(Duration::from_secs(10), socket.recv())
        .await?
        .ok_or_else(|| anyhow::anyhow!("closed during authentication"))??;
    let WsMessage::Text(text) = text else {
        anyhow::bail!("expected authentication response");
    };
    let envelope: Envelope = serde_json::from_str(&text)?;
    anyhow::ensure!(envelope.v == 1, "unsupported protocol version");
    let Message::AuthResponse(response) = envelope.decode()? else {
        anyhow::bail!("expected authentication response");
    };
    let public_key: Option<String> = sqlx::query_scalar(
        "SELECT device_public_key FROM servers WHERE id=$1 AND deleted_at IS NULL",
    )
    .bind(response.server_id)
    .fetch_optional(&state.pool)
    .await?
    .flatten();
    let public_key = public_key.ok_or_else(|| anyhow::anyhow!("unknown device"))?;
    let bytes: [u8; 32] = URL_SAFE_NO_PAD
        .decode(&public_key)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid public key"))?;
    let signature = Signature::from_slice(&URL_SAFE_NO_PAD.decode(response.signature)?)?;
    VerifyingKey::from_bytes(&bytes)?.verify_strict(nonce.as_bytes(), &signature)?;
    let server_id = response.server_id;
    let session_token = auth::random_token();
    let expires_at = now_timestamp() + 3600;
    let (sender, mut receiver) = mpsc::channel::<Envelope>(32);
    let connection_id = Uuid::new_v4();
    {
        let _lifecycle = state.device_lifecycle.lock().await;
        let mut tx = state.pool.begin().await?;
        let current: Option<String> = sqlx::query_scalar(
            "SELECT device_public_key FROM servers WHERE id=$1 AND deleted_at IS NULL FOR UPDATE",
        )
        .bind(server_id)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
        anyhow::ensure!(
            current.as_deref() == Some(&public_key),
            "device was deleted or rebound during authentication"
        );
        sqlx::query("INSERT INTO sessions(token_hash,server_id,expires_at) VALUES($1,$2,$3)")
            .bind(auth::hash_token(&session_token))
            .bind(server_id)
            .bind(expires_at)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE servers SET last_seen=$2 WHERE id=$1")
            .bind(server_id)
            .bind(now_timestamp())
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        state.connections.write().await.insert(
            server_id,
            AgentConnection {
                id: connection_id,
                sender,
            },
        );
    }
    let result: anyhow::Result<()> = async {
        send(&mut socket, Envelope::new("hello.ack", HelloAck {
            server_time: now_timestamp(), session_token, session_expires_at: expires_at,
        })?).await?;
        let (mut sink, mut stream) = socket.split();
        let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
        let mut last_received = Instant::now();
        let mut introduced = false;
        loop {
            tokio::select! {
                frame = stream.next() => {
                    let Some(frame) = frame else { break; };
                    let frame = frame?;
                    last_received = Instant::now();
                    match frame {
                        WsMessage::Text(text) => {
                            let envelope: Envelope = serde_json::from_str(&text)?;
                            anyhow::ensure!(envelope.v == 1, "unsupported protocol version");
                            let message = envelope.decode()?;
                            if !introduced {
                                let Message::Hello(hello) = &message else { anyhow::bail!("expected hello"); };
                                anyhow::ensure!((sinan_protocol::PROTOCOL_MIN..=sinan_protocol::PROTOCOL_MAX).contains(&hello.protocol_version), "unsupported protocol version");
                                introduced = true;
                            }
                            process_message(&state, server_id, message).await?;
                        }
                        WsMessage::Ping(bytes) => sink.send(WsMessage::Pong(bytes)).await?,
                        WsMessage::Close(_) => break,
                        WsMessage::Pong(_) => {},
                        _ => anyhow::bail!("expected text message"),
                    }
                    sqlx::query("UPDATE servers SET last_seen=$2 WHERE id=$1 AND deleted_at IS NULL").bind(server_id).bind(now_timestamp()).execute(&state.pool).await?;
                }
                notification = receiver.recv() => {
                    let Some(notification) = notification else { break; };
                    sink.send(WsMessage::Text(serde_json::to_string(&notification)?.into())).await?;
                }
                _ = heartbeat.tick() => {
                    if last_received.elapsed() > Duration::from_secs(60) || now_timestamp() >= expires_at { break; }
                    sink.send(WsMessage::Ping(Vec::new().into())).await?;
                }
            }
        }
        Ok(())
    }.await;
    let _lifecycle = state.device_lifecycle.lock().await;
    let mut connections = state.connections.write().await;
    if connections
        .get(&server_id)
        .is_some_and(|entry| entry.id == connection_id)
    {
        connections.remove(&server_id);
        sqlx::query("UPDATE servers SET last_seen=$2 WHERE id=$1")
            .bind(server_id)
            .bind(now_timestamp() - 61)
            .execute(&state.pool)
            .await?;
    }
    result
}

async fn send(socket: &mut WebSocket, envelope: Envelope) -> anyhow::Result<()> {
    socket
        .send(WsMessage::Text(serde_json::to_string(&envelope)?.into()))
        .await?;
    Ok(())
}

pub async fn notify(state: &AppState, server_id: i64, envelope: Envelope) {
    let sender = state
        .connections
        .read()
        .await
        .get(&server_id)
        .map(|entry| entry.sender.clone());
    if let Some(sender) = sender {
        let _ = sender.try_send(envelope);
    }
}

async fn reconcile_hint(
    state: &AppState,
    server_id: i64,
    applied: AppliedRevisions,
) -> anyhow::Result<()> {
    let mut tx = state.pool.begin().await?;
    let rows = sqlx::query(
        "SELECT module,target_rev,applied_rev FROM server_module_status WHERE server_id=$1 FOR UPDATE",
    )
    .bind(server_id)
    .fetch_all(&mut *tx)
    .await?;
    let mut differs = false;
    for row in rows {
        let module: String = row.get("module");
        let reported = applied.get(&module).copied().unwrap_or(0);
        let known = row.get::<i64, _>("applied_rev") as u64;
        differs |= reported != row.get::<i64, _>("target_rev") as u64 || reported != known;
        if reported > known
            && let Ok(rev) = i64::try_from(reported)
        {
            let published: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM deployments WHERE server_id=$1 AND module=$2 AND rev=$3)")
                .bind(server_id).bind(&module).bind(rev).fetch_one(&mut *tx).await?;
            if published {
                // Applied checkpoints are durable on the device even if their result is lost.
                sqlx::query("UPDATE server_module_status SET applied_rev=$3,healthy=true,updated_at=$4 WHERE server_id=$1 AND module=$2")
                    .bind(server_id).bind(&module).bind(rev).bind(now_timestamp()).execute(&mut *tx).await?;
            }
        }
    }
    tx.commit().await?;
    if differs {
        let rev: i64 = sqlx::query_scalar("SELECT manifest_rev FROM servers WHERE id=$1")
            .bind(server_id)
            .fetch_one(&state.pool)
            .await?;
        notify(
            state,
            server_id,
            Envelope::new("manifest.changed", ManifestChanged { rev: rev as u64 })?,
        )
        .await;
    }
    Ok(())
}

pub async fn process_message(
    state: &AppState,
    server_id: i64,
    message: Message,
) -> anyhow::Result<()> {
    match message {
        Message::Hello(hello) => {
            let capabilities: Vec<_> = hello
                .capabilities
                .into_iter()
                .filter(|value| value.len() <= 128)
                .take(64)
                .collect();
            sqlx::query("UPDATE servers SET capabilities=$2 WHERE id=$1 AND deleted_at IS NULL")
                .bind(server_id)
                .bind(serde_json::to_value(capabilities)?)
                .execute(&state.pool)
                .await?;
            reconcile_hint(state, server_id, hello.applied).await?;
        }
        Message::Heartbeat(heartbeat) => {
            reconcile_hint(state, server_id, heartbeat.applied).await?
        }
        Message::TelemetryStatic(info) => {
            sqlx::query("UPDATE servers SET static_info=$2 WHERE id=$1")
                .bind(server_id)
                .bind(serde_json::to_value(info)?)
                .execute(&state.pool)
                .await?;
        }
        Message::TelemetryMetrics(metrics) => {
            let value = serde_json::to_value(metrics)?;
            let mut tx = state.pool.begin().await?;
            sqlx::query("UPDATE servers SET latest_metrics=$2,metrics_sampled_at=$3 WHERE id=$1")
                .bind(server_id)
                .bind(&value)
                .bind(sinan_protocol::telemetry::now_millis())
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO metrics_minutely(server_id,bucket,metrics) VALUES($1,$2,$3) ON CONFLICT(server_id,bucket) DO UPDATE SET metrics=EXCLUDED.metrics").bind(server_id).bind(now_timestamp()/60*60).bind(value).execute(&mut *tx).await?;
            sqlx::query("DELETE FROM metrics_minutely WHERE bucket<$1")
                .bind(now_timestamp() - 7 * 86400)
                .execute(&mut *tx)
                .await?;
            tx.commit().await?;
        }
        Message::ApplyResult(result) => record_apply_result(state, server_id, result).await?,
        Message::UsageBatch(batch) => crate::usage::ingest(state, server_id, batch).await?,
        Message::RetirementResult(result) => {
            crate::retirement::record_result(state, server_id, result).await?
        }
        Message::Unknown { message_type, .. } => {
            tracing::debug!(%message_type,"ignoring unknown device message")
        }
        _ => tracing::debug!("ignoring unsupported device message direction"),
    }
    Ok(())
}

pub async fn record_apply_result(
    state: &AppState,
    server_id: i64,
    result: ApplyResult,
) -> anyhow::Result<()> {
    let rev = i64::try_from(result.rev)?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM deployments WHERE server_id=$1 AND module=$2 AND rev=$3)",
    )
    .bind(server_id)
    .bind(&result.module)
    .bind(rev)
    .fetch_one(&state.pool)
    .await?;
    anyhow::ensure!(exists, "unpublished revision");
    let applied = result.status == ApplyStatus::Applied;
    anyhow::ensure!(!applied || result.healthy, "applied result must be healthy");
    sqlx::query("UPDATE server_module_status SET applied_rev=CASE WHEN $4 THEN GREATEST(applied_rev,$3) ELSE applied_rev END,last_result_rev=$3,healthy=$5,last_error=$6,updated_at=$7 WHERE server_id=$1 AND module=$2 AND last_result_rev<=$3 AND applied_rev<=$3")
        .bind(server_id).bind(result.module).bind(rev).bind(applied).bind(result.healthy).bind(result.error.map(|error| error.chars().take(2048).collect::<String>())).bind(now_timestamp()).execute(&state.pool).await?;
    Ok(())
}

pub async fn manifest(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Manifest>> {
    let server_id = auth::require_agent(&state, &headers).await?;
    artifacts::require_signed_agent(&state, server_id).await?;
    let row = sqlx::query("SELECT manifest_rev,static_info FROM servers WHERE id=$1")
        .bind(server_id)
        .fetch_one(&state.pool)
        .await?;
    let rev: i64 = row.get("manifest_rev");
    let mut modules = BTreeMap::new();
    let deployment=sqlx::query("SELECT rev,bundle_sha256 FROM deployments WHERE server_id=$1 AND module='singbox' ORDER BY rev DESC LIMIT 1").bind(server_id).fetch_optional(&state.pool).await?;
    if let Some(deployment) = deployment {
        let info: serde_json::Value = row.get("static_info");
        let arch = match info["arch"].as_str() {
            Some("aarch64" | "arm64") => "arm64",
            Some("x86_64" | "amd64") => "amd64",
            _ => return Err(ApiError::BadRequest("设备架构未知".into())),
        };
        let libc = info["runtime_libc"]
            .as_str()
            .or_else(|| info["libc"].as_str());
        let target = info["os"]
            .as_str()
            .and_then(|os| sinan_protocol::platform::artifact_target(os, libc, arch));
        if info["os"].is_string() && target.is_none() {
            return Err(ApiError::BadRequest("设备平台或 libc 未受支持".into()));
        }
        let artifact = if let Some(target) = target {
            match artifacts::descriptor(&state, "sing-box", "1.14.2", &target).await {
                Ok(artifact) => artifact,
                Err(ApiError::NotFound)
                    if info["os"] == "linux" && matches!(libc, Some("gnu" | "glibc")) =>
                {
                    artifacts::descriptor(&state, "sing-box", "1.14.2", arch).await?
                }
                Err(error) => return Err(error),
            }
        } else {
            artifacts::descriptor(&state, "sing-box", "1.14.2", arch).await?
        };
        let config_rev: i64 = deployment.get("rev");
        modules.insert(
            "singbox".into(),
            ModuleManifest {
                kernel_version: "1.14.2".into(),
                artifact,
                config_rev: config_rev as u64,
                bundle_url: format!(
                    "{}/api/agent/v1/bundles/{config_rev}",
                    state.config.public_url
                ),
                bundle_sha256: deployment.get("bundle_sha256"),
                stats_listen: "127.0.0.1:18085".into(),
            },
        );
    }
    Ok(Json(Manifest {
        rev: rev as u64,
        modules,
    }))
}

pub async fn bundle(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(rev): Path<i64>,
) -> ApiResult<Response> {
    let server_id = auth::require_agent(&state, &headers).await?;
    let bundle: Option<String> = sqlx::query_scalar(
        "SELECT bundle FROM deployments WHERE server_id=$1 AND module='singbox' AND rev=$2",
    )
    .bind(server_id)
    .bind(rev)
    .fetch_optional(&state.pool)
    .await?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
        ],
        bundle.ok_or(ApiError::NotFound)?,
    )
        .into_response())
}
