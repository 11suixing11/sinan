#![forbid(unsafe_code)]

use sinan_adapter_sdk::{Counter, Plan, Prepared, Privileged, RuntimeSpec};
use sinan_agent_core::{
    config::Config,
    fake::{FakeAdapter, FakeServiceManager},
    reconcile::{ApplyIntent, Reconciler},
    state::{IntentRecord, SharedState, State},
    system::SystemOps,
};
use sinan_protocol::ApplyStatus;
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    sync::{atomic::Ordering, Arc, Mutex},
    time::Duration,
};
use uuid::Uuid;

struct Fixture {
    directory: PathBuf,
    config: Config,
    state: SharedState,
    adapter: Arc<FakeAdapter>,
    services: Arc<FakeServiceManager>,
    reconciler: Arc<Reconciler>,
}

impl Fixture {
    fn new() -> Self {
        let directory = std::env::temp_dir().join(format!("sinan-reconcile-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let config = Config {
            panel_url: "http://127.0.0.1:8080".into(),
            identity_dir: directory.join("identity"),
            state_db: directory.join("state.db"),
            runtime_root: directory.join("runtime"),
            install_root: directory.join("install"),
            status_socket: directory.join("status.sock"),
            operation_timeout_secs: 1,
        };
        let state = Arc::new(Mutex::new(State::open(&config.state_db).unwrap()));
        let adapter = Arc::new(FakeAdapter::default());
        let services = Arc::new(FakeServiceManager::default());
        let reconciler = Arc::new(Reconciler::new(
            config.clone(),
            state.clone(),
            adapter.clone(),
            Arc::new(SystemOps),
            services.clone(),
        ));
        Self {
            directory,
            config,
            state,
            adapter,
            services,
            reconciler,
        }
    }
    fn target(&self, rev: u64, kernel: &str, hash: &str) -> Prepared {
        let revision_dir = self
            .config
            .runtime_root
            .join("demo@main/revisions")
            .join(rev.to_string());
        let kernel_dir = self.config.install_root.join("demo").join(kernel);
        fs::create_dir_all(&revision_dir).unwrap();
        fs::create_dir_all(&kernel_dir).unwrap();
        fs::write(revision_dir.join("config.json"), "{}").unwrap();
        fs::write(kernel_dir.join("demo"), b"fake executable").unwrap();
        Prepared {
            spec: RuntimeSpec {
                revision: rev,
                kernel_version: kernel.into(),
                config_hash: hash.into(),
                binary_path: kernel_dir.join("demo"),
                revision_dir,
                stats_listen: "127.0.0.1:18085".into(),
                files: BTreeMap::from([("config.json".into(), "{}".into())]),
            },
            listen_ports: vec![],
        }
    }
    fn applied(&self) -> Option<Prepared> {
        self.state.lock().unwrap().get_json("applied:demo").unwrap()
    }
    fn set_counter(&self, up: u64, down: u64) {
        *self.adapter.counters.lock().unwrap() = vec![Counter {
            stat_name: "u1_n3".into(),
            uplink: up,
            downlink: down,
        }];
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

#[tokio::test]
async fn changed_config_reloads_kernel_change_restarts_and_same_hash_is_noop() {
    let test = Fixture::new();
    for target in [
        test.target(1, "1", "a"),
        test.target(2, "1", "a"),
        test.target(3, "1", "b"),
        test.target(4, "2", "b"),
    ] {
        assert_eq!(
            test.reconciler.apply_prepared(target).await.unwrap().status,
            ApplyStatus::Applied
        );
    }
    assert_eq!(
        *test.services.actions.lock().unwrap(),
        ["restart:demo@main", "reload:demo@main", "restart:demo@main"]
    );
    assert_eq!(test.applied().unwrap().spec.revision, 4);
    assert!(test
        .state
        .lock()
        .unwrap()
        .pending_intents()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn failed_health_rolls_back_links_state_and_captures_terminal_usage() {
    let test = Fixture::new();
    let previous = test.target(1, "1", "a");
    test.reconciler
        .apply_prepared(previous.clone())
        .await
        .unwrap();
    test.set_counter(100, 200);
    test.reconciler.sample_usage().await.unwrap();
    test.set_counter(150, 280);
    test.adapter.fail_health.store(true, Ordering::SeqCst);
    let result = test
        .reconciler
        .apply_prepared(test.target(2, "2", "b"))
        .await
        .unwrap();
    assert_eq!(result.status, ApplyStatus::Failed);
    assert!(result.healthy);
    assert_eq!(test.applied(), Some(previous.clone()));
    assert_eq!(
        fs::read_link(test.config.runtime_root.join("demo@main/current")).unwrap(),
        previous.spec.revision_dir
    );
    assert_eq!(
        fs::read_link(test.config.install_root.join("demo/current")).unwrap(),
        previous.spec.binary_path.parent().unwrap()
    );
    test.set_counter(10, 20);
    test.reconciler.sample_usage().await.unwrap();
    let batches = test.state.lock().unwrap().pending_usage().unwrap();
    assert_eq!(
        batches
            .iter()
            .flat_map(|batch| &batch.records)
            .map(|record| record.uplink)
            .sum::<u64>(),
        160
    );
    assert_eq!(
        batches
            .iter()
            .flat_map(|batch| &batch.records)
            .map(|record| record.downlink)
            .sum::<u64>(),
        300
    );
    assert!(test
        .state
        .lock()
        .unwrap()
        .pending_intents()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn missing_terminal_counters_prevent_service_disruption() {
    let test = Fixture::new();
    test.reconciler
        .apply_prepared(test.target(1, "1", "a"))
        .await
        .unwrap();
    let actions = test.services.actions.lock().unwrap().clone();
    test.adapter.fail_counters.store(true, Ordering::SeqCst);
    let result = test
        .reconciler
        .apply_prepared(test.target(2, "1", "b"))
        .await
        .unwrap();
    assert_eq!(result.status, ApplyStatus::Failed);
    assert_eq!(*test.services.actions.lock().unwrap(), actions);
    assert_eq!(test.applied().unwrap().spec.revision, 1);
    assert!(test
        .state
        .lock()
        .unwrap()
        .pending_intents()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn unfinished_intent_is_recovered_after_database_reopen() {
    let test = Fixture::new();
    let previous = test.target(1, "1", "a");
    test.reconciler
        .apply_prepared(previous.clone())
        .await
        .unwrap();
    let target = test.target(2, "2", "b");
    let op_id = Uuid::new_v4();
    test.state
        .lock()
        .unwrap()
        .begin_intent(&IntentRecord {
            op_id,
            module: "demo".into(),
            payload: serde_json::to_value(ApplyIntent {
                previous: Some(previous.clone()),
                target: target.clone(),
                plan: Plan::Restart,
            })
            .unwrap(),
        })
        .unwrap();
    SystemOps
        .atomic_symlink(
            &test.config.runtime_root.join("demo@main/current"),
            &target.spec.revision_dir,
        )
        .await
        .unwrap();
    SystemOps
        .atomic_symlink(
            &test.config.install_root.join("demo/current"),
            target.spec.binary_path.parent().unwrap(),
        )
        .await
        .unwrap();
    let reopened = Arc::new(Mutex::new(State::open(&test.config.state_db).unwrap()));
    let restarted = Reconciler::new(
        test.config.clone(),
        reopened.clone(),
        test.adapter.clone(),
        Arc::new(SystemOps),
        test.services.clone(),
    );
    restarted.recover().await.unwrap();
    assert!(reopened
        .lock()
        .unwrap()
        .pending_intents()
        .unwrap()
        .is_empty());
    assert_eq!(
        reopened
            .lock()
            .unwrap()
            .get_json::<Prepared>("applied:demo")
            .unwrap(),
        Some(previous.clone())
    );
    assert_eq!(
        fs::read_link(test.config.runtime_root.join("demo@main/current")).unwrap(),
        previous.spec.revision_dir
    );
}

#[tokio::test]
async fn queued_versions_are_coalesced_to_the_latest_after_current_apply() {
    let test = Fixture::new();
    test.adapter.apply_delay_ms.store(100, Ordering::SeqCst);
    let first = test.target(1, "1", "a");
    let middle = test.target(2, "1", "b");
    let latest = test.target(3, "1", "c");
    let runner = test.reconciler.clone();
    let first = tokio::spawn(async move { runner.apply_prepared(first).await.unwrap() });
    tokio::time::timeout(Duration::from_secs(1), async {
        while test.adapter.applied.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let runner = test.reconciler.clone();
    let middle = tokio::spawn(async move { runner.apply_prepared(middle).await.unwrap() });
    tokio::time::sleep(Duration::from_millis(10)).await;
    let runner = test.reconciler.clone();
    let latest = tokio::spawn(async move { runner.apply_prepared(latest).await.unwrap() });
    assert_eq!(first.await.unwrap().rev, 1);
    assert_eq!(middle.await.unwrap().rev, 3);
    assert_eq!(latest.await.unwrap().rev, 3);
    assert_eq!(*test.adapter.applied.lock().unwrap(), [1, 3]);
}

#[tokio::test]
async fn timed_out_initial_apply_stops_runtime_and_clears_intent() {
    let test = Fixture::new();
    test.adapter.apply_delay_ms.store(1500, Ordering::SeqCst);
    let result = test
        .reconciler
        .apply_prepared(test.target(1, "1", "a"))
        .await
        .unwrap();
    assert_eq!(result.status, ApplyStatus::Failed);
    assert!(!result.healthy);
    assert!(result.error.unwrap().contains("timed out"));
    assert!(test.applied().is_none());
    assert!(test
        .state
        .lock()
        .unwrap()
        .pending_intents()
        .unwrap()
        .is_empty());
    assert!(!test.services.active.load(Ordering::SeqCst));
}
