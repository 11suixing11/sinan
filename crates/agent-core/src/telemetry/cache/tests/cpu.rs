use super::*;
use std::sync::atomic::AtomicBool;

struct Release(Arc<AtomicBool>);
impl Drop for Release {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

struct BusySource {
    calls: Arc<AtomicUsize>,
    release: Arc<AtomicBool>,
}

impl Source for BusySource {
    fn collect(&mut self) -> Result<(StaticInfo, Metrics)> {
        if self.calls.fetch_add(1, Ordering::SeqCst) != 0 {
            // Exercise a synchronous CPU poll, rather than a blocked channel or
            // an async future that would voluntarily yield to the executor.
            while !self.release.load(Ordering::Acquire) {
                std::hint::spin_loop();
            }
        }
        Ok((StaticInfo::default(), Metrics::default()))
    }
}

#[tokio::test(flavor = "current_thread")]
async fn busy_sampling_preserves_executor_progress_and_bounded_shutdown() -> Result<()> {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let release = Release(Arc::new(AtomicBool::new(false)));
    let stop = release.0.clone();
    let sampling = Sampling::start_with(
        move || {
            Ok(Box::new(BusySource {
                calls: count,
                release: stop,
            }))
        },
        Control {
            interval: Duration::from_millis(10),
            offset_ms: 0,
            minimum_timestamp: 0,
            enabled: true,
        },
    )?;
    tokio::time::timeout(Duration::from_secs(2), async {
        while calls.load(Ordering::SeqCst) < 2 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?;
    let before = sampling.snapshots.borrow().sample.clone();
    tokio::time::timeout(
        Duration::from_secs(1),
        tokio::time::sleep(Duration::from_millis(20)),
    )
    .await?;
    assert_eq!(sampling.snapshots.borrow().sample, before);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    let started = Instant::now();
    drop(sampling);
    assert!(started.elapsed() < Duration::from_millis(100));
    drop(release);
    Ok(())
}
