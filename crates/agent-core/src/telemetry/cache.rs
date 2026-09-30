use super::{Collector, hardware};
use anyhow::{Context, Result};
use sinan_adapter_sdk::Privileged;
use sinan_protocol::{Metrics, StaticInfo, TelemetrySample, telemetry::now_millis};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
use tokio::sync::watch;
use uuid::Uuid;

const COLLECTION_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone)]
pub(crate) struct Snapshot {
    pub static_info: StaticInfo,
    pub sample: Option<TelemetrySample>,
    pub started_at: Option<Instant>,
    pub error: Option<String>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Self {
            static_info: StaticInfo {
                os: Some(std::env::consts::OS.into()),
                libc: cfg!(target_os = "linux").then(|| {
                    if cfg!(target_env = "musl") {
                        "musl"
                    } else {
                        "gnu"
                    }
                    .into()
                }),
                arch: Some(std::env::consts::ARCH.into()),
                ..StaticInfo::default()
            },
            sample: None,
            started_at: None,
            error: None,
        }
    }
}

impl Snapshot {
    pub fn timed_out(&self) -> bool {
        self.started_at
            .is_some_and(|time| time.elapsed() >= COLLECTION_TIMEOUT)
    }
}

#[derive(Clone)]
pub(crate) struct Control {
    pub interval: Duration,
    pub offset_ms: i64,
    pub minimum_timestamp: i64,
    pub enabled: bool,
}

pub(crate) trait Source {
    fn collect(&mut self) -> Result<(StaticInfo, Metrics)>;
}

struct NativeSource {
    collector: Collector,
    ops: Arc<dyn Privileged>,
    runtime: tokio::runtime::Runtime,
    previous: (Instant, BTreeMap<String, [u64; 7]>),
    supplement: hardware::Hardware,
    refreshed: Option<Instant>,
}

impl Source for NativeSource {
    fn collect(&mut self) -> Result<(StaticInfo, Metrics)> {
        if self
            .refreshed
            .is_none_or(|time| time.elapsed() >= Duration::from_secs(5))
        {
            self.supplement = self
                .runtime
                .block_on(hardware::refresh(self.ops.as_ref(), &mut self.previous))?;
            self.refreshed = Some(Instant::now());
        }
        let mut metrics = self.collector.metrics();
        self.supplement.apply(&mut metrics);
        Ok((self.collector.static_info(), metrics))
    }
}

pub(crate) struct Sampling {
    // Keep the last snapshot available even if the collecting thread panics.
    _snapshot: watch::Sender<Arc<Snapshot>>,
    pub snapshots: watch::Receiver<Arc<Snapshot>>,
    pub control: watch::Sender<Control>,
    stopped: Arc<AtomicBool>,
    wake: mpsc::Sender<()>,
}

impl Sampling {
    pub fn start(ops: Arc<dyn Privileged>, control: Control) -> Result<Self> {
        Self::start_with(
            move || {
                Ok(Box::new(NativeSource {
                    collector: Collector::new(),
                    ops,
                    runtime: tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?,
                    previous: (Instant::now(), BTreeMap::new()),
                    supplement: Default::default(),
                    refreshed: None,
                }))
            },
            control,
        )
    }

    pub(crate) fn start_with(
        factory: impl FnOnce() -> Result<Box<dyn Source>> + Send + 'static,
        control: Control,
    ) -> Result<Self> {
        let (snapshot_tx, snapshots) = watch::channel(Arc::new(Snapshot::default()));
        let (control_tx, control_rx) = watch::channel(control);
        let stopped = Arc::new(AtomicBool::new(false));
        let (wake, wait) = mpsc::channel();
        let stop = stopped.clone();
        let output = snapshot_tx.clone();
        // Exactly one thread owns the collector for this Agent process. A timeout
        // reports stale data; it never starts another possibly blocked collector.
        std::thread::Builder::new()
            .name("sinan-telemetry".into())
            .spawn(move || {
                let mut cached = Snapshot {
                    started_at: Some(Instant::now()),
                    ..Snapshot::default()
                };
                output.send_replace(Arc::new(cached.clone()));
                let mut source = match factory() {
                    Ok(source) => source,
                    Err(error) => {
                        cached.error = Some(error.to_string());
                        output.send_replace(Arc::new(cached));
                        return;
                    }
                };
                let mut last_timestamp: i64 = 0;
                while !stop.load(Ordering::Relaxed) {
                    let control = control_rx.borrow().clone();
                    if control.enabled {
                        let started = Instant::now();
                        let timestamp = now_millis()
                            .saturating_add(control.offset_ms)
                            .max(last_timestamp.saturating_add(1))
                            .max(control.minimum_timestamp.saturating_add(1));
                        cached.started_at = Some(started);
                        output.send_replace(Arc::new(cached.clone()));
                        let collected = source.collect();
                        if stop.load(Ordering::Relaxed) {
                            break;
                        }
                        match collected {
                            Ok((info, metrics)) if started.elapsed() < COLLECTION_TIMEOUT => {
                                last_timestamp = timestamp;
                                cached.static_info = info;
                                cached.sample = Some(TelemetrySample {
                                    id: Uuid::new_v4(),
                                    sampled_at: timestamp,
                                    metrics,
                                });
                                cached.error = None;
                            }
                            Ok(_) => {
                                cached.error =
                                    Some("telemetry collection exceeded five seconds".into())
                            }
                            Err(error) => cached.error = Some(error.to_string()),
                        }
                        cached.started_at = None;
                        output.send_replace(Arc::new(cached.clone()));
                    }
                    if wait.recv_timeout(control.interval).is_ok() {
                        break;
                    }
                }
            })
            .context("start isolated telemetry thread")?;
        Ok(Self {
            _snapshot: snapshot_tx,
            snapshots,
            control: control_tx,
            stopped,
            wake,
        })
    }
}

impl Drop for Sampling {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        let _ = self.wake.send(());
        // Joining a collector stuck in a kernel filesystem call would stall shutdown.
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    pub struct BlockingFixture {
        pub sampling: Sampling,
        pub calls: Arc<AtomicUsize>,
        pub starts: Arc<AtomicUsize>,
        release: mpsc::Sender<()>,
    }

    impl BlockingFixture {
        pub fn new() -> Result<Self> {
            let (release, block) = mpsc::channel();
            let calls = Arc::new(AtomicUsize::new(0));
            let starts = Arc::new(AtomicUsize::new(0));
            let count = calls.clone();
            let start = starts.clone();
            let sampling = Sampling::start_with(
                move || {
                    start.fetch_add(1, Ordering::SeqCst);
                    Ok(Box::new(BlockingSource {
                        calls: count,
                        block,
                    }))
                },
                Control {
                    interval: Duration::from_millis(10),
                    offset_ms: 0,
                    minimum_timestamp: 0,
                    enabled: true,
                },
            )?;
            Ok(Self {
                sampling,
                calls,
                starts,
                release,
            })
        }

        pub async fn wait_until_blocked(&self) -> Result<TelemetrySample> {
            tokio::time::timeout(Duration::from_secs(2), async {
                while self.calls.load(Ordering::SeqCst) < 2 {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await?;
            Ok(self.sampling.snapshots.borrow().sample.clone().unwrap())
        }
    }

    impl Drop for BlockingFixture {
        fn drop(&mut self) {
            let _ = self.release.send(());
        }
    }

    struct BlockingSource {
        calls: Arc<AtomicUsize>,
        block: mpsc::Receiver<()>,
    }

    impl Source for BlockingSource {
        fn collect(&mut self) -> Result<(StaticInfo, Metrics)> {
            if self.calls.fetch_add(1, Ordering::SeqCst) > 0 {
                self.block.recv()?;
            }
            Ok((
                StaticInfo {
                    hostname: Some("fixture-cached-host".into()),
                    ..Default::default()
                },
                Metrics {
                    uptime_secs: Some(42),
                    ..Default::default()
                },
            ))
        }
    }

    #[tokio::test]
    async fn blocked_collector_keeps_snapshot_identity_and_drop_does_not_wait() -> Result<()> {
        let fixture = BlockingFixture::new()?;
        let previous = fixture.wait_until_blocked().await?;
        tokio::time::sleep(COLLECTION_TIMEOUT + Duration::from_millis(100)).await;
        let snapshot = fixture.sampling.snapshots.borrow().clone();
        assert!(snapshot.timed_out());
        assert_eq!(snapshot.sample.as_ref(), Some(&previous));
        assert_eq!(fixture.starts.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.calls.load(Ordering::SeqCst), 2);
        let started = Instant::now();
        drop(fixture);
        assert!(started.elapsed() < Duration::from_millis(100));
        Ok(())
    }

    #[tokio::test]
    async fn blocked_constructor_preserves_platform_identity_without_sampling_on_executor()
    -> Result<()> {
        let (release, wait) = mpsc::channel();
        let sampling = Sampling::start_with(
            move || {
                wait.recv()?;
                Ok(Box::new(ImmediateSource))
            },
            Control {
                interval: Duration::from_secs(1),
                offset_ms: 0,
                minimum_timestamp: 0,
                enabled: true,
            },
        )?;
        let mut snapshots = sampling.snapshots.clone();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                snapshots.changed().await?;
                if snapshots.borrow().started_at.is_some() {
                    break Ok::<_, anyhow::Error>(());
                }
            }
        })
        .await??;
        let info = snapshots.borrow().static_info.clone();
        assert_eq!(info.os.as_deref(), Some(std::env::consts::OS));
        assert_eq!(info.arch.as_deref(), Some(std::env::consts::ARCH));
        if cfg!(target_os = "linux") {
            assert!(info.libc.is_some());
        }
        tokio::time::sleep(COLLECTION_TIMEOUT + Duration::from_millis(100)).await;
        assert!(snapshots.borrow().timed_out());
        assert!(snapshots.borrow().sample.is_none());
        let started = Instant::now();
        drop(sampling);
        assert!(started.elapsed() < Duration::from_millis(100));
        release.send(())?;
        Ok(())
    }

    struct ImmediateSource;
    impl Source for ImmediateSource {
        fn collect(&mut self) -> Result<(StaticInfo, Metrics)> {
            Ok((StaticInfo::default(), Metrics::default()))
        }
    }

    #[tokio::test]
    async fn clock_correction_and_restart_floor_preserve_monotonic_sample_time() -> Result<()> {
        let floor = now_millis();
        let sampling = Sampling::start_with(
            || Ok(Box::new(ImmediateSource)),
            Control {
                interval: Duration::from_millis(10),
                offset_ms: -1_000_000,
                minimum_timestamp: floor,
                enabled: true,
            },
        )?;
        let mut snapshots = sampling.snapshots.clone();
        let first = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                snapshots.changed().await?;
                if let Some(sample) = snapshots.borrow().sample.clone() {
                    break Ok::<_, anyhow::Error>(sample);
                }
            }
        })
        .await??;
        assert_eq!(first.sampled_at, floor + 1);
        sampling
            .control
            .send_modify(|control| control.minimum_timestamp = floor + 100);
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                snapshots.changed().await?;
                if snapshots
                    .borrow()
                    .sample
                    .as_ref()
                    .is_some_and(|sample| sample.sampled_at > floor + 100)
                {
                    break Ok::<_, anyhow::Error>(());
                }
            }
        })
        .await??;
        Ok(())
    }
}
