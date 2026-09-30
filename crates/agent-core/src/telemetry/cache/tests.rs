use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

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
async fn blocked_constructor_preserves_platform_identity_without_sampling_on_executor() -> Result<()>
{
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

#[cfg(unix)]
mod cleanup;
mod cpu;
mod recovery;
