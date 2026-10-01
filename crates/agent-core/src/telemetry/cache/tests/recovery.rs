use super::*;

struct RecoveringSource {
    calls: Arc<AtomicUsize>,
    started: Arc<tokio::sync::Notify>,
    resume: mpsc::Receiver<()>,
}

impl Source for RecoveringSource {
    fn collect(&mut self) -> Result<(StaticInfo, Metrics)> {
        match self.calls.fetch_add(1, Ordering::SeqCst) {
            1 => anyhow::bail!("TEST_ONLY transient sampling failure"),
            2 => {
                self.started.notify_one();
                self.resume.recv()?;
            }
            _ => {}
        }
        Ok((
            StaticInfo::default(),
            Metrics {
                uptime_secs: Some(42),
                ..Default::default()
            },
        ))
    }
}

#[tokio::test]
async fn failed_and_slow_sampling_retains_identity_until_fresh_recovery() -> Result<()> {
    let calls = Arc::new(AtomicUsize::new(0));
    let count = calls.clone();
    let started = Arc::new(tokio::sync::Notify::new());
    let collecting = started.clone();
    let (release, resume) = mpsc::channel();
    let sampling = Sampling::start_with(
        move || {
            Ok(Box::new(RecoveringSource {
                calls: count,
                started: collecting,
                resume,
            }))
        },
        Control {
            interval: Duration::from_millis(10),
            offset_ms: 0,
            minimum_timestamp: 0,
            enabled: true,
        },
    )?;
    let mut snapshots = sampling.snapshots.clone();
    // Starting collect publishes the retained snapshot before incrementing the
    // source call counter. Wait for the actual blocked collection, not a race
    // between two independently observed watch/counter updates.
    tokio::time::timeout(Duration::from_secs(2), started.notified()).await?;
    let before = {
        let snapshot = snapshots.borrow().clone();
        assert!(snapshot.error.is_some());
        assert!(calls.load(Ordering::SeqCst) >= 3);
        snapshot.sample.clone().unwrap()
    };
    tokio::time::sleep(COLLECTION_TIMEOUT + Duration::from_millis(100)).await;
    assert!(snapshots.borrow().timed_out());
    assert_eq!(snapshots.borrow().sample.as_ref(), Some(&before));
    release.send(())?;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            snapshots.changed().await?;
            let snapshot = snapshots.borrow().clone();
            if snapshot
                .sample
                .as_ref()
                .is_some_and(|sample| sample.id != before.id)
            {
                assert!(snapshot.sample.as_ref().unwrap().sampled_at > before.sampled_at);
                assert!(snapshot.error.is_none());
                assert!(!snapshot.timed_out());
                assert!(calls.load(Ordering::SeqCst) >= 4);
                break Ok::<_, anyhow::Error>(());
            }
        }
    })
    .await??;
    Ok(())
}
