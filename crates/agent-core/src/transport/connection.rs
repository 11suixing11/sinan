use super::Runtime;
use crate::{
    artifacts::PanelClient, config::validate_panel_url, identity::Identity, telemetry::Collector,
    Config,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::Signer;
use futures_util::{SinkExt, StreamExt};
use sinan_protocol::{
    AuthResponse, Envelope, Heartbeat, Hello, HelloAck, Message, PROTOCOL_VERSION,
};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};
use tokio::{
    net::TcpStream,
    sync::{mpsc, watch},
    time::{timeout, Instant},
};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::{protocol::WebSocketConfig, Message as Frame},
    MaybeTlsStream, WebSocketStream,
};

type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(super) async fn run(
    config: &Config,
    identity: &Identity,
    runtime: &Runtime,
    client_tx: &watch::Sender<Option<Arc<PanelClient>>>,
    trigger: &mpsc::Sender<()>,
    outgoing: &mut mpsc::Receiver<Envelope>,
) -> Result<()> {
    let (mut socket, ack) = authenticate(config, identity).await?;
    send(
        &mut socket,
        Envelope::new(
            "hello",
            Hello {
                agent_version: env!("CARGO_PKG_VERSION").into(),
                protocol_version: PROTOCOL_VERSION,
                capabilities: runtime.modules.as_ref().clone(),
                applied: runtime.applied()?,
            },
        )?,
    )
    .await?;
    let mut collector = Collector::new();
    send(
        &mut socket,
        Envelope::new("telemetry.static", runtime.static_info()?)?,
    )
    .await?;
    client_tx.send_replace(Some(Arc::new(PanelClient::new(
        &config.panel_url,
        &ack.session_token,
    )?)));
    runtime.connected.store(true, Ordering::Relaxed);
    let _ = trigger.try_send(());
    let renew_after = ack
        .session_expires_at
        .saturating_sub(ack.server_time)
        .saturating_sub(60)
        .clamp(1, 3540) as u64;
    let renewal = tokio::time::sleep(Duration::from_secs(renew_after));
    tokio::pin!(renewal);
    let mut heartbeat = interval(20);
    let mut telemetry = interval(10);
    let mut resend = interval(15);
    let mut stale = interval(10);
    let mut last_received = Instant::now();
    let mut resend_queue = std::collections::VecDeque::new();
    loop {
        tokio::select! {
            biased;
            incoming = socket.next() => {
                let frame = incoming.context("panel WebSocket closed")??;
                last_received = Instant::now();
                match frame {
                    Frame::Text(text) => {
                        let envelope: Envelope = serde_json::from_str(&text)?;
                        anyhow::ensure!(envelope.v == PROTOCOL_VERSION, "incompatible panel protocol version");
                        match envelope.decode()? {
                            Message::ManifestChanged(_) => { let _ = trigger.try_send(()); }
                            Message::UsageAck(ack) => {
                                runtime.state.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                                    .acknowledge_usage(ack.epoch, ack.seq)?;
                            }
                            Message::Unknown { message_type, .. } => tracing::debug!(%message_type, "ignoring unknown panel message"),
                            _ => tracing::debug!("ignoring unsupported panel message direction"),
                        }
                    }
                    Frame::Ping(bytes) => { timeout(Duration::from_secs(10), socket.send(Frame::Pong(bytes))).await??; }
                    Frame::Pong(_) => {}
                    Frame::Close(_) => return Ok(()),
                    _ => anyhow::bail!("panel sent an unsupported frame"),
                }
            }
            message = outgoing.recv() => {
                send(&mut socket, message.context("runtime result channel closed")?).await?;
            }
            _ = heartbeat.tick() => {
                send(&mut socket, Envelope::new("heartbeat", Heartbeat {
                    applied: runtime.applied()?, uptime_secs: sysinfo::System::uptime(),
                })?).await?;
            }
            _ = telemetry.tick() => {
                send(&mut socket, Envelope::new("telemetry.metrics", collector.metrics())?).await?;
            }
            _ = resend.tick() => {
                if resend_queue.is_empty() {
                    let pending = runtime.state.lock().map_err(|_| anyhow::anyhow!("state lock poisoned"))?.pending_usage()?;
                    resend_queue.extend(pending.into_iter().take(64));
                }
            }
            _ = stale.tick() => {
                anyhow::ensure!(last_received.elapsed() <= Duration::from_secs(60), "panel connection timed out");
            }
            _ = &mut renewal => {
                let _ = timeout(Duration::from_secs(2), socket.close(None)).await;
                return Ok(());
            }
            _ = std::future::ready(()), if !resend_queue.is_empty() => {
                // Yield to input and timers between batches even when catching up after an outage.
                let batch = resend_queue.pop_front().context("resend queue unexpectedly empty")?;
                send(&mut socket, Envelope::new("usage.batch", batch)?).await?;
            }
        }
    }
}

fn interval(seconds: u64) -> tokio::time::Interval {
    let mut timer = tokio::time::interval(Duration::from_secs(seconds));
    timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    timer
}

async fn authenticate(config: &Config, identity: &Identity) -> Result<(Socket, HelloAck)> {
    let mut url = validate_panel_url(&config.panel_url)?.join("api/agent/v1/ws")?;
    let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
    url.set_scheme(scheme)
        .map_err(|_| anyhow::anyhow!("invalid WebSocket URL"))?;
    let options = WebSocketConfig::default()
        .max_message_size(Some(1024 * 1024))
        .max_frame_size(Some(1024 * 1024));
    let (mut socket, _) = timeout(
        Duration::from_secs(config.operation_timeout_secs),
        connect_async_with_config(url.as_str(), Some(options), false),
    )
    .await??;
    let challenge = receive(&mut socket).await?;
    let Message::AuthChallenge(challenge) = challenge.decode()? else {
        anyhow::bail!("expected panel authentication challenge");
    };
    anyhow::ensure!(
        !challenge.nonce.is_empty() && challenge.nonce.len() <= 512,
        "invalid authentication challenge"
    );
    let signature = identity.signing_key.sign(challenge.nonce.as_bytes());
    send(
        &mut socket,
        Envelope::new(
            "auth.response",
            AuthResponse {
                server_id: identity.server_id,
                signature: URL_SAFE_NO_PAD.encode(signature.to_bytes()),
            },
        )?,
    )
    .await?;
    let ack = receive(&mut socket).await?;
    let Message::HelloAck(ack) = ack.decode()? else {
        anyhow::bail!("panel rejected authentication");
    };
    anyhow::ensure!(
        !ack.session_token.is_empty()
            && ack.session_token.len() <= 512
            && ack.session_expires_at > ack.server_time,
        "invalid panel session"
    );
    Ok((socket, ack))
}

async fn receive(socket: &mut Socket) -> Result<Envelope> {
    timeout(Duration::from_secs(10), async {
        loop {
            match socket
                .next()
                .await
                .context("panel closed authentication connection")??
            {
                Frame::Text(text) => {
                    let envelope: Envelope = serde_json::from_str(&text)?;
                    anyhow::ensure!(
                        envelope.v == PROTOCOL_VERSION,
                        "incompatible panel protocol version"
                    );
                    return Ok(envelope);
                }
                Frame::Ping(bytes) => socket.send(Frame::Pong(bytes)).await?,
                Frame::Pong(_) => {}
                _ => anyhow::bail!("expected authentication envelope"),
            }
        }
    })
    .await?
}

async fn send(socket: &mut Socket, envelope: Envelope) -> Result<()> {
    let encoded = serde_json::to_string(&envelope)?;
    anyhow::ensure!(
        encoded.len() < 1024 * 1024,
        "outgoing message exceeds protocol limit"
    );
    timeout(
        Duration::from_secs(10),
        socket.send(Frame::Text(encoded.into())),
    )
    .await??;
    Ok(())
}
