use super::*;
use sinan_adapter_sdk::{BoxFuture, CommandOutput};
use std::{
    path::Path,
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

#[derive(Default)]
struct ControlledOps {
    active: AtomicUsize,
    peak: AtomicUsize,
    calls: AtomicUsize,
}

struct Active<'a>(&'a AtomicUsize);
impl Drop for Active<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

impl Privileged for ControlledOps {
    fn execute<'a>(&'a self, _: &'a Path, args: &'a [String]) -> BoxFuture<'a, CommandOutput> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            let _active = Active(&self.active);
            if !args.last().unwrap().contains("127.0.0.1") {
                std::future::pending::<()>().await;
            }
            Ok(CommandOutput {
                success: true,
                stdout: if cfg!(windows) {
                    r#"{"sent":4,"times":[0,0,0,0]}"#
                } else {
                    "4 packets transmitted, 4 received\nrtt min/avg/max = 0/0/0 ms"
                }
                .into(),
                stderr: String::new(),
            })
        })
    }
    fn create_dir<'a>(&'a self, _: &'a Path, _: u32, _: Option<&'a str>) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected filesystem operation") })
    }
    fn write_file<'a>(
        &'a self,
        _: &'a Path,
        _: &'a [u8],
        _: u32,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected filesystem operation") })
    }
    fn atomic_symlink<'a>(&'a self, _: &'a Path, _: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected filesystem operation") })
    }
    fn remove_symlink<'a>(&'a self, _: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected filesystem operation") })
    }
    fn install_archive<'a>(&'a self, _: &'a Path, _: &'a Path, _: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected filesystem operation") })
    }
}

async fn until(condition: impl Fn() -> bool) -> Result<()> {
    timeout(Duration::from_secs(5), async {
        while !condition() {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .context("probe scheduler did not make progress")
}

#[tokio::test]
async fn slow_probes_do_not_delay_results_and_configuration_changes_cancel_work() -> Result<()> {
    let state = Arc::new(Mutex::new(crate::State::open(Path::new(":memory:"))?));
    let ops = Arc::new(ControlledOps::default());
    let retirement = Arc::new(crate::retirement::Retirement::new(
        crate::Config::default(),
        state.clone(),
        vec![],
        ops.clone(),
        Arc::new(crate::fake::FakeServiceManager::default()),
    )?);
    let mut specs: Vec<_> = (1..=6)
        .map(|id| ProbeSpec {
            id: Uuid::from_u128(id),
            name: format!("fixture-{id}"),
            kind: ProbeKind::Icmp,
            target: if id == 1 { "127.0.0.1" } else { "127.0.0.2" }.into(),
            port: None,
            interval_secs: 3600,
            carrier: String::new(),
            monitor: None,
            execution_authorized: None,
            enabled: true,
        })
        .collect();
    for spec in &mut specs {
        authorize_fixture(spec);
    }
    state
        .lock()
        .unwrap()
        .set_json("probes:configuration", &(now_timestamp(), &specs))?;
    let worker = tokio::spawn(sample_loop(state.clone(), ops.clone(), retirement.clone()));
    until(|| state.lock().unwrap().probe_results().unwrap().len() == 1).await?;
    until(|| ops.active.load(Ordering::SeqCst) == 4).await?;
    // Reconfigure a completed target before its hour-long interval, freeing one slot.
    specs[0].name = "changed".into();
    specs[1].enabled = false;
    specs[5].enabled = false;
    state
        .lock()
        .unwrap()
        .set_json("probes:configuration", &(now_timestamp(), &specs))?;
    until(|| state.lock().unwrap().probe_results().unwrap().len() == 2).await?;
    assert!(ops.peak.load(Ordering::SeqCst) <= 4);
    for spec in &mut specs {
        spec.enabled = false;
    }
    state
        .lock()
        .unwrap()
        .set_json("probes:configuration", &(now_timestamp(), &specs))?;
    until(|| ops.active.load(Ordering::SeqCst) == 0).await?;
    // Cancelled measurements release the gate used by retirement and persist no results.
    let guard = timeout(Duration::from_secs(1), retirement.gate.write()).await?;
    assert_eq!(state.lock().unwrap().probe_results()?.len(), 2);
    drop(guard);
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    Ok(())
}

#[tokio::test]
async fn cached_legacy_targets_are_inert_and_revocation_aborts_active_measurement() -> Result<()> {
    let state = Arc::new(Mutex::new(crate::State::open(Path::new(":memory:"))?));
    let ops = Arc::new(ControlledOps::default());
    let retirement = Arc::new(crate::retirement::Retirement::new(
        crate::Config::default(),
        state.clone(),
        vec![],
        ops.clone(),
        Arc::new(crate::fake::FakeServiceManager::default()),
    )?);
    let mut spec = ProbeSpec {
        id: Uuid::new_v4(),
        name: "cached legacy fixture".into(),
        kind: ProbeKind::Icmp,
        target: "127.0.0.2".into(),
        port: None,
        interval_secs: 3600,
        carrier: String::new(),
        enabled: true,
        monitor: None,
        execution_authorized: Some(true),
    };
    state.lock().unwrap().set_json(
        "probes:configuration",
        &(now_timestamp(), vec![spec.clone()]),
    )?;
    let worker = tokio::spawn(sample_loop(state.clone(), ops.clone(), retirement.clone()));
    tokio::time::sleep(Duration::from_millis(250)).await;
    assert_eq!(ops.active.load(Ordering::SeqCst), 0);
    assert!(state.lock().unwrap().probe_results()?.is_empty());
    authorize_fixture(&mut spec);
    state.lock().unwrap().set_json(
        "probes:configuration",
        &(now_timestamp(), vec![spec.clone()]),
    )?;
    until(|| ops.active.load(Ordering::SeqCst) == 1).await?;
    spec.monitor
        .as_mut()
        .unwrap()
        .authorization
        .as_mut()
        .unwrap()
        .enabled = false;
    state
        .lock()
        .unwrap()
        .set_json("probes:configuration", &(now_timestamp(), vec![spec]))?;
    until(|| ops.active.load(Ordering::SeqCst) == 0).await?;
    assert!(state.lock().unwrap().probe_results()?.is_empty());
    let guard = timeout(Duration::from_secs(1), retirement.gate.write()).await?;
    drop(guard);
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    Ok(())
}

#[tokio::test]
async fn authorization_expiry_cancels_a_cached_measurement_without_panel_refresh() -> Result<()> {
    let state = Arc::new(Mutex::new(crate::State::open(Path::new(":memory:"))?));
    let ops = Arc::new(ControlledOps::default());
    let retirement = Arc::new(crate::retirement::Retirement::new(
        crate::Config::default(),
        state.clone(),
        vec![],
        ops.clone(),
        Arc::new(crate::fake::FakeServiceManager::default()),
    )?);
    let mut spec = ProbeSpec {
        id: Uuid::new_v4(),
        name: "expiring fixture".into(),
        kind: ProbeKind::Icmp,
        target: "127.0.0.2".into(),
        port: None,
        interval_secs: 3600,
        carrier: String::new(),
        enabled: true,
        monitor: None,
        execution_authorized: None,
    };
    authorize_fixture(&mut spec);
    spec.monitor
        .as_mut()
        .unwrap()
        .authorization
        .as_mut()
        .unwrap()
        .expires_at = Some(now_timestamp() + 3);
    state
        .lock()
        .unwrap()
        .set_json("probes:configuration", &(now_timestamp(), vec![spec]))?;
    let worker = tokio::spawn(sample_loop(state.clone(), ops.clone(), retirement.clone()));
    until(|| ops.active.load(Ordering::SeqCst) == 1).await?;
    until(|| ops.active.load(Ordering::SeqCst) == 0).await?;
    assert!(state.lock().unwrap().probe_results()?.is_empty());
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    Ok(())
}

#[test]
fn expired_leases_future_cache_times_and_invalid_authorizations_fail_closed() -> Result<()> {
    let mut state = crate::State::open(Path::new(":memory:"))?;
    let mut spec: ProbeSpec = serde_json::from_value(
        serde_json::json!({"id":Uuid::new_v4(),"name":"TEST_ONLY","kind":"tcp","target":"127.0.0.1","port":443,"interval_secs":30,"carrier":"","enabled":true}),
    )?;
    authorize_fixture(&mut spec);
    state.set_json(
        "probes:configuration",
        &(now_timestamp(), vec![spec.clone()]),
    )?;
    assert_eq!(leased_configuration(&state)?, vec![spec.clone()]);
    for fetched in [
        now_timestamp() - CONFIGURATION_LEASE_SECS,
        now_timestamp() + 3600,
    ] {
        state.set_json("probes:configuration", &(fetched, vec![spec.clone()]))?;
        assert!(leased_configuration(&state)?.is_empty());
    }
    let mut unauthorized = spec;
    unauthorized
        .monitor
        .as_mut()
        .unwrap()
        .authorization
        .as_mut()
        .unwrap()
        .scope
        .clear();
    state.set_json(
        "probes:configuration",
        &(now_timestamp(), vec![unauthorized]),
    )?;
    assert!(leased_configuration(&state)?.is_empty());
    state.set_json(
        "probes:configuration",
        &"TEST_ONLY invalid cached structure",
    )?;
    assert!(leased_configuration(&state).is_err());
    Ok(())
}

#[tokio::test]
async fn lease_expiry_cancels_work_and_persistence_failure_does_not_stop_the_scheduler()
-> Result<()> {
    let state = Arc::new(Mutex::new(crate::State::open(Path::new(":memory:"))?));
    let ops = Arc::new(ControlledOps::default());
    let retirement = Arc::new(crate::retirement::Retirement::new(
        crate::Config::default(),
        state.clone(),
        vec![],
        ops.clone(),
        Arc::new(crate::fake::FakeServiceManager::default()),
    )?);
    let mut spec: ProbeSpec = serde_json::from_value(
        serde_json::json!({"id":Uuid::new_v4(),"name":"TEST_ONLY","kind":"icmp","target":"127.0.0.2","port":null,"interval_secs":30,"carrier":"","enabled":true}),
    )?;
    authorize_fixture(&mut spec);
    state.lock().unwrap().set_json(
        "probes:configuration",
        &(now_timestamp(), vec![spec.clone()]),
    )?;
    let worker = tokio::spawn(sample_loop(state.clone(), ops.clone(), retirement));
    until(|| ops.active.load(Ordering::SeqCst) == 1).await?;
    state.lock().unwrap().set_json(
        "probes:configuration",
        &(
            now_timestamp() - CONFIGURATION_LEASE_SECS,
            vec![spec.clone()],
        ),
    )?;
    until(|| ops.active.load(Ordering::SeqCst) == 0).await?;
    assert!(state.lock().unwrap().probe_results()?.is_empty());
    assert!(!worker.is_finished());
    // A storage failure is isolated to the bounded probe outbox, not task lifetime.
    {
        let state = state.lock().unwrap();
        state.connection.execute_batch("CREATE TABLE TEST_ONLY_storage_fill(payload BLOB); CREATE TRIGGER TEST_ONLY_probe_storage_full BEFORE INSERT ON probe_outbox BEGIN INSERT INTO TEST_ONLY_storage_fill VALUES(zeroblob(65536)); END;")?;
        let pages: i64 = state
            .connection
            .query_row("PRAGMA page_count", [], |row| row.get(0))?;
        state
            .connection
            .execute_batch(&format!("PRAGMA max_page_count={pages}"))?;
    }
    spec.target = "127.0.0.1".into();
    authorize_fixture(&mut spec);
    state.lock().unwrap().set_json(
        "probes:configuration",
        &(now_timestamp(), vec![spec.clone()]),
    )?;
    until(|| ops.calls.load(Ordering::SeqCst) == 2).await?;
    spec.name = "TEST_ONLY after storage failure".into();
    state
        .lock()
        .unwrap()
        .set_json("probes:configuration", &(now_timestamp(), vec![spec]))?;
    until(|| ops.calls.load(Ordering::SeqCst) == 3).await?;
    tokio::task::yield_now().await;
    assert!(!worker.is_finished());
    assert!(state.lock().unwrap().probe_results()?.is_empty());
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    Ok(())
}
