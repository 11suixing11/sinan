use super::*;
use crate::{State, fake::FakeServiceManager, system::SystemOps};
use sinan_protocol::{Metrics, TelemetrySample};
use std::{path::Path, sync::Mutex};
use uuid::Uuid;

#[tokio::test]
async fn full_storage_keeps_worker_alive_and_retries_the_same_snapshot_after_recovery() -> Result<()>
{
    let config = Config::default();
    let mut stored = State::open(Path::new(":memory:"))?;
    stored.set_json("agent_settings", &config.settings)?;
    let old = TelemetrySample {
        id: Uuid::new_v4(),
        sampled_at: 1000,
        metrics: Metrics::default(),
    };
    stored.save_telemetry(&old)?;
    let pages: i64 = stored
        .connection
        .query_row("PRAGMA page_count", [], |row| row.get(0))?;
    stored
        .connection
        .execute_batch(&format!("PRAGMA max_page_count={pages}"))?;
    let state = Arc::new(Mutex::new(stored));
    let retirement = Arc::new(crate::retirement::Retirement::new(
        config.clone(),
        state.clone(),
        vec![],
        Arc::new(SystemOps),
        Arc::new(FakeServiceManager::default()),
    )?);
    let mut sample = TelemetrySample {
        id: Uuid::new_v4(),
        sampled_at: 2000,
        metrics: Metrics::default(),
    };
    sample.metrics.extra.insert(
        "TEST_ONLY_bounded_payload".into(),
        serde_json::json!("x".repeat(60 * 1024)),
    );
    let (snapshots, receiver) = watch::channel(Arc::new(Snapshot::default()));
    let (control, mut controls) = watch::channel(initial_control(&config, &state)?);
    let (_clients, client) = watch::channel(None);
    let worker = tokio::spawn(run(
        config.clone(),
        state.clone(),
        receiver,
        control,
        client,
        retirement,
    ));
    snapshots.send_replace(Arc::new(Snapshot {
        sample: Some(sample.clone()),
        ..Snapshot::default()
    }));
    tokio::time::timeout(Duration::from_secs(2), controls.changed()).await??;
    // A second cycle has observed the actual FULL error and remains responsive.
    tokio::time::timeout(Duration::from_secs(2), controls.changed()).await??;
    assert!(!worker.is_finished());
    {
        let stored = state
            .try_lock()
            .expect("state lock is released between cycles");
        assert_eq!(stored.pending_telemetry()?, vec![old.clone()]);
        assert_eq!(stored.latest_telemetry_timestamp()?, old.sampled_at);
        assert_eq!(
            stored.get_json::<AgentSettings>("agent_settings")?,
            Some(config.settings)
        );
        stored
            .connection
            .execute_batch("PRAGMA max_page_count=100000")?;
    }
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if state
                .lock()
                .unwrap()
                .pending_telemetry()?
                .iter()
                .any(|item| item == &sample)
            {
                break Ok::<_, anyhow::Error>(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await??;
    assert_eq!(
        state.lock().unwrap().pending_telemetry()?,
        vec![old, sample]
    );
    assert!(!worker.is_finished());
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    Ok(())
}
