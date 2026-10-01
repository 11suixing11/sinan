use super::*;
use crate::{
    State, artifacts::PanelClient, fake::FakeServiceManager, retirement::Retirement,
    system::SystemOps,
};
use sinan_protocol::{Metrics, TelemetryBatch, TelemetrySample, telemetry::now_millis};
use std::{
    io::Read,
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Notify, Semaphore},
};
use uuid::Uuid;

#[tokio::test]
async fn live_delivery_remains_responsive_while_durable_ack_is_blocked() -> Result<()> {
    delivery_fixture(false).await
}

#[tokio::test]
async fn old_panels_keep_the_legacy_durable_upload_schedule() -> Result<()> {
    delivery_fixture(true).await
}

async fn delivery_fixture(legacy: bool) -> Result<()> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let started = Arc::new(Notify::new());
    let release = Arc::new(Semaphore::new(0));
    let first = Arc::new(AtomicBool::new(true));
    let live = Arc::new(Mutex::new(Vec::<Uuid>::new()));
    let server = {
        let started = started.clone();
        let release = release.clone();
        let live = live.clone();
        tokio::spawn(async move {
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let started = started.clone();
                let release = release.clone();
                let first = first.clone();
                let live = live.clone();
                tokio::spawn(async move {
                    let mut bytes = Vec::new();
                    let mut buffer = [0u8; 4096];
                    let (header_end, length) = loop {
                        let read = socket.read(&mut buffer).await.unwrap();
                        if read == 0 {
                            return;
                        }
                        bytes.extend_from_slice(&buffer[..read]);
                        if let Some(end) = bytes.windows(4).position(|part| part == b"\r\n\r\n") {
                            let headers = String::from_utf8_lossy(&bytes[..end]);
                            let length = headers
                                .lines()
                                .find_map(|line| {
                                    line.to_ascii_lowercase()
                                        .strip_prefix("content-length:")
                                        .map(|value| value.trim().parse::<usize>().unwrap())
                                })
                                .unwrap_or(0);
                            break (end + 4, length);
                        }
                    };
                    while bytes.len() < header_end + length {
                        let read = socket.read(&mut buffer).await.unwrap();
                        bytes.extend_from_slice(&buffer[..read]);
                    }
                    let header = String::from_utf8_lossy(&bytes[..header_end]);
                    let route = header
                        .lines()
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .nth(1)
                        .unwrap();
                    let body=match route {
                        "/api/agent/v1/settings"=>serde_json::json!({"sample_interval_secs":1,"upload_interval_secs":1}),
                        "/api/agent/v1/telemetry-settings"=>serde_json::json!({"persist_interval_secs":15}),
                        "/api/agent/v1/telemetry/live"=>{
                            let sample:TelemetrySample=serde_json::from_slice(&bytes[header_end..]).unwrap();
                            live.lock().unwrap().push(sample.id);
                            serde_json::json!({"accepted":true,"sampled_at":sample.sampled_at})
                        },
                        "/api/agent/v1/telemetry"=>{
                            let mut decoded=Vec::new();
                            flate2::read::GzDecoder::new(&bytes[header_end..]).read_to_end(&mut decoded).unwrap();
                            let batch:TelemetryBatch=serde_json::from_slice(&decoded).unwrap();
                            if first.swap(false,Ordering::SeqCst) {started.notify_one();release.acquire().await.unwrap().forget();}
                            serde_json::json!({"ids":batch.samples.iter().map(|sample|sample.id).collect::<Vec<_>>()})
                        },
                        _=>panic!("unexpected fixture request"),
                    }.to_string();
                    let status = if legacy && route == "/api/agent/v1/telemetry-settings" {
                        "404 Not Found"
                    } else {
                        "200 OK"
                    };
                    socket.write_all(format!("HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
                });
            }
        })
    };
    let state = Arc::new(Mutex::new(State::open(Path::new(":memory:"))?));
    let config = Config {
        settings: AgentSettings {
            sample_interval_secs: 1,
            upload_interval_secs: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    let retirement = Arc::new(Retirement::new(
        config.clone(),
        state.clone(),
        Vec::new(),
        Arc::new(SystemOps),
        Arc::new(FakeServiceManager::default()),
    )?);
    let sample = |at| TelemetrySample {
        id: Uuid::new_v4(),
        sampled_at: at,
        metrics: Metrics::default(),
    };
    let first_sample = sample(now_millis());
    state.lock().unwrap().save_telemetry(&first_sample)?;
    let (snapshots_tx, snapshots) = watch::channel(Arc::new(Snapshot::default()));
    let (control, _) = watch::channel(initial_control(&config, &state)?);
    let (_clients_tx, clients) = watch::channel(Some(Arc::new(PanelClient::new(
        &format!("http://{address}"),
        "TEST_ONLY_SESSION",
    )?)));
    let worker = tokio::spawn(run(
        config,
        state.clone(),
        snapshots,
        control,
        clients,
        retirement.clone(),
    ));
    tokio::time::timeout(Duration::from_secs(5), started.notified()).await?;
    let second = sample(now_millis() + 1);
    snapshots_tx.send_replace(Arc::new(Snapshot {
        sample: Some(second.clone()),
        ..Default::default()
    }));
    tokio::time::timeout(Duration::from_secs(5), async {
        while if legacy {
            state.lock().unwrap().pending_telemetry_count().unwrap() < 2
        } else {
            !live.lock().unwrap().contains(&second.id)
        } {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await?;
    assert_eq!(state.lock().unwrap().pending_telemetry_count()?, 2);
    if legacy {
        assert!(live.lock().unwrap().is_empty());
    }
    let writer_queued = Arc::new(Notify::new());
    let writer_acquired = Arc::new(Notify::new());
    let (unlock, unlocked) = tokio::sync::oneshot::channel();
    let writer = {
        let queued = writer_queued.clone();
        let acquired = writer_acquired.clone();
        tokio::spawn(async move {
            queued.notify_one();
            let _guard = retirement.gate.write().await;
            acquired.notify_one();
            let _ = unlocked.await;
        })
    };
    writer_queued.notified().await;
    release.add_permits(1);
    tokio::time::timeout(Duration::from_secs(5), writer_acquired.notified()).await?;
    // A retirement writer takes priority between ACKs; the second batch must
    // still be durable locally while that writer holds the gate.
    assert_eq!(state.lock().unwrap().pending_telemetry_count()?, 1);
    unlock.send(()).unwrap();
    writer.await?;
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if state.lock().unwrap().pending_telemetry_count()? == 0 {
                break Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    worker.abort();
    server.abort();
    Ok(())
}
