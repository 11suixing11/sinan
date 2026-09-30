use super::{Collector, hardware};
use crate::{Config, SharedState};
use anyhow::{Result, ensure};
use sinan_adapter_sdk::Privileged;
use sinan_protocol::{AgentSettings, TelemetrySample, telemetry::now_millis};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

pub async fn run(
    config: Config,
    state: SharedState,
    ops: Arc<dyn Privileged>,
    mut client: tokio::sync::watch::Receiver<Option<Arc<crate::artifacts::PanelClient>>>,
) -> Result<()> {
    let hardware = Arc::new(Mutex::new(hardware::Hardware::default()));
    let collect = async {
        let mut collector = Collector::new();
        let mut last_timestamp = 0;
        loop {
            let settings = state
                .lock()
                .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                .get_json::<AgentSettings>("agent_settings")?
                .unwrap_or_else(|| config.settings.clone());
            ensure!(settings.valid(), "invalid telemetry settings");
            let mut metrics = collector.metrics();
            let supplement = hardware
                .lock()
                .map_err(|_| anyhow::anyhow!("hardware lock poisoned"))?
                .clone();
            metrics.gpus = supplement.gpus;
            if let Some((tcp, udp)) = supplement.connections {
                metrics.tcp_connections = Some(tcp);
                metrics.udp_connections = Some(udp);
            }
            for disk in &mut metrics.disks {
                let name = std::path::Path::new(&disk.name)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                if let Some(io) = supplement.io.get(name.as_ref()) {
                    disk.read_bytes_per_sec = io.read_bytes_per_sec;
                    disk.write_bytes_per_sec = io.write_bytes_per_sec;
                    disk.read_iops = io.read_iops;
                    disk.write_iops = io.write_iops;
                    disk.await_ms = io.await_ms;
                    disk.utilization_percent = io.utilization_percent;
                }
            }
            {
                let mut state = state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                let offset = state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0);
                let timestamp = now_millis().saturating_add(offset).max(last_timestamp + 1);
                last_timestamp = timestamp;
                let dropped = state.save_telemetry(&TelemetrySample {
                    id: Uuid::new_v4(),
                    sampled_at: timestamp,
                    metrics,
                })?;
                if dropped > 0 {
                    tracing::warn!(
                        dropped,
                        "oldest telemetry samples removed by offline retention limit"
                    );
                }
            }
            tokio::time::sleep(Duration::from_secs(settings.sample_interval_secs)).await;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    let refresh = async {
        let mut previous = (Instant::now(), hardware::disk_snapshot());
        loop {
            let supplement = hardware::refresh(ops.as_ref(), &mut previous).await?;
            *hardware
                .lock()
                .map_err(|_| anyhow::anyhow!("hardware lock poisoned"))? = supplement;
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    let upload = async {
        let mut last_settings: Option<Instant> = None;
        loop {
            let active = client.borrow_and_update().clone();
            if let Some(active) = active {
                if last_settings.is_none_or(|v| v.elapsed() >= Duration::from_secs(60)) {
                    last_settings = Some(Instant::now());
                    match active.agent_settings().await {
                        Ok(settings) => state
                            .lock()
                            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                            .set_json("agent_settings", &settings)?,
                        Err(error) => tracing::warn!(%error, "Agent settings refresh failed"),
                    }
                }
                let samples = state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                    .pending_telemetry()?;
                if !samples.is_empty() {
                    match active
                        .telemetry(&sinan_protocol::TelemetryBatch { samples })
                        .await
                    {
                        Ok(ack) => state
                            .lock()
                            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                            .acknowledge_telemetry(&ack)?,
                        Err(error) => {
                            tracing::warn!(%error, "telemetry upload failed; samples retained for retry")
                        }
                    }
                }
            }
            let settings = state
                .lock()
                .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                .get_json::<AgentSettings>("agent_settings")?
                .unwrap_or_else(|| config.settings.clone());
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(settings.upload_interval_secs)) => {},
                changed = client.changed() => { changed?; last_settings = None; },
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    tokio::try_join!(collect, refresh, upload)?;
    Ok(())
}
