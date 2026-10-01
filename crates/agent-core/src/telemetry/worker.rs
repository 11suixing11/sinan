use super::cache::{Control, Snapshot};
use crate::{Config, SharedState, state::StorageRetry};
use anyhow::{Result, ensure};
use sinan_protocol::AgentSettings;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::watch;

#[cfg(test)]
mod tests;

pub(crate) fn initial_control(config: &Config, state: &SharedState) -> Result<Control> {
    let state = state
        .lock()
        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
    let settings = state
        .get_json::<AgentSettings>("agent_settings")?
        .unwrap_or_else(|| config.settings.clone());
    ensure!(settings.valid(), "invalid telemetry settings");
    Ok(Control {
        interval: Duration::from_secs(settings.sample_interval_secs),
        offset_ms: state.get_json::<i64>("clock_offset_ms")?.unwrap_or(0),
        minimum_timestamp: state.latest_telemetry_timestamp()?,
        enabled: true,
    })
}

pub(crate) async fn run(
    config: Config,
    state: SharedState,
    mut snapshots: watch::Receiver<Arc<Snapshot>>,
    control: watch::Sender<Control>,
    mut client: watch::Receiver<Option<Arc<crate::artifacts::PanelClient>>>,
    retirement: Arc<crate::retirement::Retirement>,
) -> Result<()> {
    // Persistence and upload never own or call the collector. Even a kernel call
    // that never returns leaves both async futures and the connection responsive.
    let persist = async {
        let mut saved = None;
        let mut stale = false;
        let mut storage = StorageRetry::default();
        let mut poll = tokio::time::interval(Duration::from_secs(1));
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            {
                let _guard = retirement.gate.read().await;
                let mut next_control = initial_control(&config, &state)?;
                next_control.enabled = !retirement.requested();
                control.send_replace(next_control);
                let snapshot = snapshots.borrow_and_update().clone();
                let expired = snapshot.timed_out() || snapshot.error.is_some();
                if expired && !stale {
                    tracing::warn!(error = ?snapshot.error, "telemetry collector stalled; last sample retained without refreshing its time");
                }
                stale = expired;
                if let Some(sample) = snapshot
                    .sample
                    .as_ref()
                    .filter(|sample| !retirement.requested() && Some(sample.id) != saved)
                {
                    let persisted = {
                        let mut state = state
                            .lock()
                            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                        storage.finish("persist telemetry", state.save_telemetry(sample))?
                    };
                    if let Some(dropped) = persisted {
                        saved = Some(sample.id);
                        if dropped > 0 {
                            tracing::warn!(
                                dropped,
                                "oldest telemetry samples removed by offline retention limit"
                            );
                        }
                    }
                }
            }
            tokio::select! {
                _ = poll.tick() => {},
                changed = snapshots.changed() => { changed?; },
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    let upload = async {
        let mut last_settings: Option<Instant> = None;
        let mut settings_storage = StorageRetry::default();
        let mut acknowledgment_storage = StorageRetry::default();
        loop {
            let interval = {
                let _guard = retirement.gate.read().await;
                if retirement.requested() {
                    config.settings.upload_interval_secs
                } else {
                    let active = client.borrow_and_update().clone();
                    if let Some(active) = active {
                        if last_settings.is_none_or(|v| v.elapsed() >= Duration::from_secs(60)) {
                            last_settings = Some(Instant::now());
                            match active.agent_settings().await {
                                Ok(settings) => {
                                    let mut state = state
                                        .lock()
                                        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                                    if settings_storage
                                        .finish(
                                            "persist Agent settings",
                                            state.set_json("agent_settings", &settings),
                                        )?
                                        .is_none()
                                    {
                                        // Keep the last durable settings. The next upload cycle retries.
                                        last_settings = None;
                                    }
                                }
                                Err(error) => {
                                    tracing::warn!(%error, "Agent settings refresh failed")
                                }
                            }
                        }
                        let samples = state
                            .lock()
                            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                            .pending_telemetry()?;
                        if !retirement.requested() && !samples.is_empty() {
                            match active
                                .telemetry(&sinan_protocol::TelemetryBatch { samples })
                                .await
                            {
                                Ok(ack) => {
                                    let mut state = state
                                        .lock()
                                        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
                                    acknowledgment_storage.finish(
                                        "persist telemetry acknowledgment",
                                        state.acknowledge_telemetry(&ack),
                                    )?;
                                }
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
                    settings.upload_interval_secs
                }
            };
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(interval)) => {},
                changed = client.changed() => { changed?; last_settings = None; },
            }
        }
        #[allow(unreachable_code)]
        Ok::<(), anyhow::Error>(())
    };
    tokio::try_join!(persist, upload)?;
    Ok(())
}
