mod connection;
mod status;
mod worker;

pub use status::status;

use crate::{artifacts::PanelClient, identity, reconcile::Reconciler, Config, SharedState, State};
use anyhow::{Context, Result};
use sinan_adapter_sdk::{Adapter, Prepared, Privileged, ServiceManager};
use sinan_protocol::AppliedRevisions;
use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    sync::{mpsc, watch},
    task::JoinSet,
    time::Instant,
};

#[derive(Clone)]
struct Runtime {
    state: SharedState,
    modules: Arc<Vec<String>>,
    connected: Arc<AtomicBool>,
}

impl Runtime {
    fn applied(&self) -> Result<AppliedRevisions> {
        let state = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?;
        let mut applied = BTreeMap::new();
        for module in self.modules.iter() {
            if let Some(prepared) = state.get_json::<Prepared>(&format!("applied:{module}"))? {
                applied.insert(module.clone(), prepared.spec.revision);
            }
        }
        Ok(applied)
    }
}

pub async fn run(
    config: Config,
    adapters: Vec<Arc<dyn Adapter>>,
    privileged: Arc<dyn Privileged>,
    services: Arc<dyn ServiceManager>,
) -> Result<()> {
    config.validate()?;
    let identity = identity::load(&config)?;
    // Reserve the instance before inspecting or recovering another process's intents.
    let listener = status::bind(&config.status_socket).await?;
    let state = Arc::new(Mutex::new(State::open(&config.state_db)?));
    let mut modules = Vec::new();
    let mut reconcilers = Vec::new();
    for adapter in adapters {
        let module = adapter.describe().module;
        anyhow::ensure!(!modules.contains(&module), "duplicate adapter module");
        modules.push(module.clone());
        let reconciler = Reconciler::new(
            config.clone(),
            state.clone(),
            adapter,
            privileged.clone(),
            services.clone(),
        );
        reconciler.recover().await?;
        reconcilers.push((module, reconciler));
    }
    let runtime = Runtime {
        state,
        modules: Arc::new(modules),
        connected: Arc::new(AtomicBool::new(false)),
    };
    let (client_tx, client_rx) = watch::channel::<Option<Arc<PanelClient>>>(None);
    let (trigger_tx, trigger_rx) = mpsc::channel(1);
    let (outgoing_tx, mut outgoing_rx) = mpsc::channel(64);
    let mut tasks = JoinSet::new();
    tasks.spawn(status::serve(listener, runtime.clone()));
    tasks.spawn(worker::run(
        reconcilers,
        runtime.clone(),
        client_rx,
        trigger_rx,
        outgoing_tx,
    ));
    let mut attempt = 0;
    loop {
        let started = Instant::now();
        let result = tokio::select! {
            result = connection::run(&config, &identity, &runtime, &client_tx, &trigger_tx, &mut outgoing_rx) => result,
            task = tasks.join_next() => {
                task.context("runtime has no background workers")???;
                anyhow::bail!("runtime worker stopped unexpectedly");
            }
        };
        runtime.connected.store(false, Ordering::Relaxed);
        client_tx.send_replace(None);
        if let Err(error) = result {
            tracing::warn!(%error, "panel connection interrupted");
        }
        if started.elapsed() >= Duration::from_secs(60) {
            attempt = 0;
        }
        let delay = reconnect_delay(attempt, rand::random());
        attempt = attempt.saturating_add(1);
        tokio::select! {
            _ = tokio::time::sleep(delay) => {},
            task = tasks.join_next() => {
                task.context("runtime has no background workers")???;
                anyhow::bail!("runtime worker stopped unexpectedly");
            }
        }
    }
}

fn reconnect_delay(attempt: u32, jitter: f64) -> Duration {
    let base = (1_u64 << attempt.min(6)).min(60);
    Duration::from_secs_f64(base as f64 * (1.0 + 0.3 * jitter.clamp(0.0, 1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        fake::{FakeAdapter, FakeServiceManager},
        reconcile::ApplyIntent,
        state::IntentRecord,
        system::SystemOps,
    };
    use sinan_adapter_sdk::{Plan, RuntimeSpec};
    use std::{os::unix::fs::PermissionsExt, path::PathBuf};
    use uuid::Uuid;

    struct Directory(PathBuf);
    impl Drop for Directory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn reconnect_backoff_is_bounded_and_jittered() {
        assert_eq!(reconnect_delay(0, 0.0), Duration::from_secs(1));
        assert_eq!(reconnect_delay(4, 0.0), Duration::from_secs(16));
        assert_eq!(reconnect_delay(100, 0.0), Duration::from_secs(60));
        assert_eq!(reconnect_delay(100, 1.0), Duration::from_secs(78));
    }

    #[tokio::test]
    async fn second_runtime_cannot_recover_an_active_instances_intent() -> Result<()> {
        let directory =
            Directory(PathBuf::from("/tmp").join(format!("sn-instance-{}", Uuid::new_v4())));
        let panel = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let config = Config {
            panel_url: format!("http://{}", panel.local_addr()?),
            identity_dir: directory.0.join("identity"),
            state_db: directory.0.join("state.db"),
            runtime_root: directory.0.join("runtime"),
            install_root: directory.0.join("install"),
            status_socket: directory.0.join("status.sock"),
            operation_timeout_secs: 1,
        };
        std::fs::create_dir_all(&config.identity_dir)?;
        std::fs::write(config.identity_dir.join("device.key"), [7_u8; 32])?;
        std::fs::set_permissions(
            config.identity_dir.join("device.key"),
            std::fs::Permissions::from_mode(0o600),
        )?;
        std::fs::write(config.identity_dir.join("server_id"), "1")?;
        std::fs::write(config.identity_dir.join("panel_origin"), &config.panel_url)?;
        let adapter = Arc::new(FakeAdapter::default());
        let services = Arc::new(FakeServiceManager::default());
        let mut tasks = JoinSet::new();
        tasks.spawn(run(
            config.clone(),
            vec![adapter.clone()],
            Arc::new(SystemOps),
            services.clone(),
        ));
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if status(&config.status_socket).await.is_ok() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await?;
        let record = IntentRecord {
            op_id: Uuid::new_v4(),
            module: "demo".into(),
            payload: serde_json::to_value(ApplyIntent {
                previous: None,
                plan: Plan::Restart,
                target: Prepared {
                    spec: RuntimeSpec {
                        revision: 1,
                        kernel_version: "1.0.0".into(),
                        config_hash: "test-hash".into(),
                        binary_path: config.install_root.join("demo/1.0.0/demo"),
                        revision_dir: config.runtime_root.join("demo@main/revisions/1"),
                        stats_listen: "127.0.0.1:18085".into(),
                        files: BTreeMap::new(),
                    },
                    listen_ports: vec![],
                },
            })?,
        };
        let mut state = State::open(&config.state_db)?;
        state.begin_intent(&record)?;
        let error = run(
            config.clone(),
            vec![adapter],
            Arc::new(SystemOps),
            services.clone(),
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("already active"));
        assert_eq!(state.pending_intents()?, vec![record]);
        assert!(services.actions.lock().unwrap().is_empty());
        assert!(status(&config.status_socket).await.is_ok());
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
        assert!(!config.status_socket.exists());
        Ok(())
    }
}
