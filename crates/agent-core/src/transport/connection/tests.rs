use super::*;
use crate::{State, usage::MAX_PENDING_USAGE_BYTES};
use ed25519_dalek::SigningKey;
use sinan_protocol::{AuthChallenge, UsageAck, UsageBatch, UsageRecord};
use std::{collections::BTreeMap, path::PathBuf, sync::Mutex};
use tokio::{net::TcpListener, task::JoinSet};
use tokio_tungstenite::accept_async;
use uuid::Uuid;

type Peer = WebSocketStream<TcpStream>;

struct Directory(PathBuf);

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn receive_peer(peer: &mut Peer) -> Result<(Envelope, usize)> {
    loop {
        match peer.next().await.context("fixture connection closed")?? {
            Frame::Text(text) => return Ok((serde_json::from_str(&text)?, text.len())),
            Frame::Ping(bytes) => peer.send(Frame::Pong(bytes)).await?,
            other => anyhow::bail!("unexpected fixture frame: {other:?}"),
        }
    }
}

async fn send_peer(peer: &mut Peer, envelope: Envelope) -> Result<()> {
    peer.send(Frame::Text(serde_json::to_string(&envelope)?.into()))
        .await?;
    Ok(())
}

async fn accept_peer(listener: &TcpListener) -> Result<Peer> {
    let (stream, _) = listener.accept().await?;
    let mut peer = accept_async(stream).await?;
    send_peer(
        &mut peer,
        Envelope::new(
            "auth.challenge",
            AuthChallenge {
                nonce: "usage-bound-fixture".into(),
                server_time: 10,
            },
        )?,
    )
    .await?;
    assert!(matches!(
        receive_peer(&mut peer).await?.0.decode()?,
        Message::AuthResponse(_)
    ));
    send_peer(
        &mut peer,
        Envelope::new(
            "hello.ack",
            HelloAck {
                server_time: 10,
                session_token: "fixture-session".into(),
                session_expires_at: 3_610,
            },
        )?,
    )
    .await?;
    Ok(peer)
}

fn spawn_connection(tasks: &mut JoinSet<Result<()>>, config: Config, runtime: Runtime) {
    tasks.spawn(async move {
        let identity = Identity {
            server_id: 1,
            signing_key: SigningKey::from_bytes(&[7; 32]),
        };
        let (client_tx, _client_rx) = watch::channel(None);
        let (trigger_tx, _trigger_rx) = mpsc::channel(1);
        let (_outgoing_tx, mut outgoing_rx) = mpsc::channel(1);
        run(
            &config,
            &identity,
            &runtime,
            &client_tx,
            &trigger_tx,
            &mut outgoing_rx,
        )
        .await
    });
}

#[tokio::test]
async fn backlog_and_legacy_giant_preserve_heartbeat_control_ack_and_restart_replay() -> Result<()>
{
    let directory =
        Directory(std::env::temp_dir().join(format!("sn-usage-wire-{}", Uuid::new_v4())));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let config = Config {
        panel_url: format!("http://{}", listener.local_addr()?),
        state_db: directory.0.join("state.db"),
        identity_dir: directory.0.join("identity"),
        runtime_root: directory.0.join("runtime"),
        install_root: directory.0.join("install"),
        agent_root: directory.0.join("core"),
        status_socket: directory.0.join("status.sock"),
        operation_timeout_secs: 1,
        public_ips: vec![],
        settings: sinan_protocol::AgentSettings::default(),
    };
    let mut state = State::open(&config.state_db)?;
    let transaction = state.connection.transaction()?;
    transaction.execute(
        "INSERT INTO usage_outbox(epoch,seq,batch) VALUES (?1,'1',?2)",
        rusqlite::params![
            Uuid::nil().to_string(),
            "x".repeat(2 * MAX_PENDING_USAGE_BYTES)
        ],
    )?;
    for seq in 2..=4_097 {
        let batch = UsageBatch {
            epoch: Uuid::from_u128(u128::from(seq)),
            seq,
            period_start: 10,
            period_end: 20,
            records: vec![UsageRecord {
                stat_name: "u1_n1".into(),
                uplink: seq,
                downlink: seq,
            }],
        };
        transaction.execute(
            "INSERT INTO usage_outbox(epoch,seq,batch) VALUES (?1,?2,?3)",
            rusqlite::params![
                batch.epoch.to_string(),
                seq.to_string(),
                serde_json::to_string(&batch)?
            ],
        )?;
    }
    transaction.commit()?;
    let mut runtime = Runtime {
        state: Arc::new(Mutex::new(state)),
        modules: Arc::new(vec![]),
        capabilities: Arc::new(vec![]),
        connected: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        public_ips: Arc::new(vec![]),
        agent_version: "fixture-agent",
        retirement: None,
    };
    let mut tasks = JoinSet::new();
    spawn_connection(&mut tasks, config.clone(), runtime.clone());
    let mut peer = timeout(Duration::from_secs(5), accept_peer(&listener)).await??;
    let mut rounds = BTreeMap::<u64, usize>::new();
    let mut window_bytes = 0;
    let mut usage_count = 0;
    let mut first_heartbeat = None;
    let mut replay = None;
    timeout(Duration::from_secs(25), async {
        loop {
            let (envelope, bytes) = receive_peer(&mut peer).await?;
            match envelope.decode()? {
                Message::UsageBatch(batch) => {
                    assert!(batch.seq >= 2);
                    window_bytes += bytes;
                    usage_count += 1;
                    *rounds.entry(batch.seq).or_default() += 1;
                    if batch.seq == 2 {
                        send_peer(
                            &mut peer,
                            Envelope::new(
                                "usage.ack",
                                UsageAck {
                                    epoch: batch.epoch,
                                    seq: batch.seq,
                                },
                            )?,
                        )
                        .await?;
                    }
                    if batch.seq == 3 {
                        replay = Some(batch);
                    }
                    if usage_count % crate::usage::MAX_PENDING_USAGE_BATCHES == 0 {
                        assert!(window_bytes <= MAX_PENDING_USAGE_BYTES);
                        window_bytes = 0;
                    }
                }
                Message::Heartbeat(_) => {
                    if let Some(first) = first_heartbeat {
                        let elapsed: Duration = Instant::now() - first;
                        assert!(elapsed <= Duration::from_secs(23));
                        assert_eq!(usage_count, 128);
                        break;
                    }
                    first_heartbeat = Some(Instant::now());
                }
                _ => {}
            }
        }
        Ok::<_, anyhow::Error>(())
    })
    .await??;
    assert_eq!(rounds.get(&2), Some(&1));
    assert_eq!(rounds.get(&3), Some(&2));
    assert!(runtime.connected.load(Ordering::Relaxed));
    assert_eq!(runtime.state.lock().unwrap().pending_usage_count()?, 4_096);
    assert_eq!(runtime.state.lock().unwrap().oversized_usage_count()?, 1);
    peer.send(Frame::Close(None)).await?;
    timeout(Duration::from_secs(5), tasks.join_next())
        .await?
        .context("connection task missing")???;
    // Reopen the persistent database just as a process restart does; the ACK survives.
    runtime.state = Arc::new(Mutex::new(State::open(&config.state_db)?));
    spawn_connection(&mut tasks, config, runtime);
    let mut peer = timeout(Duration::from_secs(5), accept_peer(&listener)).await??;
    let retried = timeout(Duration::from_secs(5), async {
        loop {
            if let Message::UsageBatch(batch) = receive_peer(&mut peer).await?.0.decode()? {
                return Ok::<_, anyhow::Error>(batch);
            }
        }
    })
    .await??;
    assert_eq!(Some(retried), replay);
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
    Ok(())
}
