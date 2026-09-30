use super::*;
use crate::{
    fake::{FakeAdapter, FakeServiceManager},
    state::IntentRecord,
    system::{SystemOps, SystemServiceManager},
};
use ed25519_dalek::{Signature, Verifier};
use sinan_adapter_sdk::{BoxFuture, CommandOutput, Counter, RuntimeSpec};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

struct Fixture {
    root: PathBuf,
    config: Config,
    state: SharedState,
    services: Arc<FakeServiceManager>,
    identity: Identity,
    retirement: Arc<Retirement>,
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
impl Fixture {
    fn new() -> Result<Self> {
        let root = std::env::temp_dir().join(format!("sinan-retirement-{}", Uuid::new_v4()));
        fs::create_dir(&root)?;
        let root = fs::canonicalize(root)?;
        let config = Config {
            panel_url: "http://127.0.0.1:9".into(),
            identity_dir: root.join("identity"),
            state_db: root.join("state.db"),
            runtime_root: root.join("runtime"),
            install_root: root.join("install"),
            status_socket: root.join("status.sock"),
            operation_timeout_secs: 1,
            public_ips: vec![],
        };
        fs::create_dir(&config.identity_dir)?;
        fs::write(config.identity_dir.join("device.key"), [19_u8; 32])?;
        fs::set_permissions(
            config.identity_dir.join("device.key"),
            fs::Permissions::from_mode(0o600),
        )?;
        fs::write(config.identity_dir.join("server_id"), "7")?;
        fs::write(config.identity_dir.join("panel_origin"), &config.panel_url)?;
        let identity = crate::identity::load(&config)?;
        let revision = config.runtime_root.join("demo@main/revisions/1");
        fs::create_dir_all(&revision)?;
        fs::write(
            revision.join("config.json"),
            "TEST_ONLY_runtime_credentials",
        )?;
        symlink(&revision, config.runtime_root.join("demo@main/current"))?;
        let mut state = State::open(&config.state_db)?;
        state.set_json(
            "applied:demo",
            &Prepared {
                spec: RuntimeSpec {
                    revision: 1,
                    kernel_version: "1.0.0".into(),
                    config_hash: "hash".into(),
                    binary_path: config.install_root.join("demo/1.0.0/demo"),
                    revision_dir: revision,
                    stats_listen: "127.0.0.1:18085".into(),
                    files: BTreeMap::from([(
                        "config.json".into(),
                        "TEST_ONLY_runtime_credentials".into(),
                    )]),
                },
                listen_ports: vec![],
            },
        )?;
        state.begin_intent(&IntentRecord {
            op_id: Uuid::new_v4(),
            module: "demo".into(),
            payload: serde_json::json!({"credentials":"TEST_ONLY_intent"}),
        })?;
        let state = Arc::new(Mutex::new(state));
        let services = Arc::new(FakeServiceManager::default());
        services.active.store(true, Ordering::SeqCst);
        let retirement = Arc::new(Retirement::new(
            config.clone(),
            state.clone(),
            vec![Arc::new(FakeAdapter::default())],
            Arc::new(SystemOps),
            services.clone(),
        )?);
        Ok(Self {
            root,
            config,
            state,
            services,
            identity,
            retirement,
        })
    }
    fn request(&self) -> Result<Uuid> {
        let id = Uuid::new_v4();
        self.retirement
            .request(&self.identity, RetirementRequest { request_id: id })?;
        Ok(id)
    }
}

#[tokio::test]
async fn request_is_durable_idempotent_and_blocks_reenrollment() -> Result<()> {
    let fixture = Fixture::new()?;
    let id = fixture.request()?;
    fixture
        .retirement
        .request(&fixture.identity, RetirementRequest { request_id: id })?;
    assert!(
        fixture
            .retirement
            .request(
                &fixture.identity,
                RetirementRequest {
                    request_id: Uuid::new_v4()
                }
            )
            .is_err()
    );
    let recovered = Retirement::new(
        fixture.config.clone(),
        fixture.state.clone(),
        vec![Arc::new(FakeAdapter::default())],
        Arc::new(SystemOps),
        fixture.services.clone(),
    )?;
    assert!(recovered.requested());
    assert!(ensure_enrollment_allowed(&fixture.config).is_err());
    assert!(!recovered.recover_completion().await?);
    assert!(!fixture.services.active.load(Ordering::SeqCst));
    assert!(
        !fixture
            .config
            .runtime_root
            .join("demo@main/current")
            .exists()
    );
    assert!(fixture.config.identity_dir.join("device.key").exists());
    Ok(())
}

#[tokio::test]
async fn shutdown_waits_for_inflight_operations_and_failure_keeps_credentials() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.request()?;
    let guard = fixture.retirement.gate.read().await;
    let retirement = fixture.retirement.clone();
    let task = tokio::spawn(async move { retirement.prepare().await });
    tokio::time::sleep(Duration::from_millis(30)).await;
    assert!(fixture.services.actions.lock().unwrap().is_empty());
    fixture.services.fail_next.store(true, Ordering::SeqCst);
    drop(guard);
    assert!(task.await?.is_err());
    assert_eq!(fixture.retirement.read()?.unwrap().phase, Phase::Requested);
    assert!(fixture.config.identity_dir.join("device.key").exists());
    fixture.retirement.prepare().await?;
    assert!(!fixture.services.active.load(Ordering::SeqCst));
    Ok(())
}

#[tokio::test]
async fn acknowledged_usage_is_preserved_while_keys_and_configuration_are_removed() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.request()?;
    let batch = fixture
        .state
        .lock()
        .unwrap()
        .sample_usage(
            "demo",
            &[Counter {
                stat_name: "u1_n1".into(),
                uplink: 31,
                downlink: 73,
            }],
            sinan_protocol::now_timestamp(),
        )?
        .unwrap();
    fixture.retirement.prepare().await?;
    assert!(
        fixture
            .retirement
            .complete(&fixture.identity)
            .await
            .is_err()
    );
    assert!(fixture.config.identity_dir.join("device.key").exists());
    fixture
        .state
        .lock()
        .unwrap()
        .acknowledge_usage(batch.epoch, batch.seq)?;
    let receipt = fixture.retirement.complete(&fixture.identity).await?;
    fixture.identity.signing_key.verifying_key().verify(
        &retirement_receipt_message(receipt.server_id, receipt.request_id),
        &Signature::from_slice(&URL_SAFE_NO_PAD.decode(&receipt.signature)?)?,
    )?;
    assert!(
        fixture
            .identity
            .signing_key
            .verifying_key()
            .verify(
                b"an-authentication-nonce",
                &Signature::from_slice(&URL_SAFE_NO_PAD.decode(&receipt.signature)?)?
            )
            .is_err()
    );
    assert!(
        fixture
            .identity
            .signing_key
            .verifying_key()
            .verify(
                &retirement_receipt_message(receipt.server_id, Uuid::new_v4()),
                &Signature::from_slice(&URL_SAFE_NO_PAD.decode(&receipt.signature)?)?
            )
            .is_err()
    );
    for name in ["device.key", "server_id", "panel_origin"] {
        assert!(!fixture.config.identity_dir.join(name).exists());
    }
    assert!(!fixture.config.runtime_root.join("demo@main").exists());
    let state = fixture.state.lock().unwrap();
    assert!(state.pending_intents()?.is_empty());
    assert!(
        state
            .get_json::<serde_json::Value>("applied:demo")?
            .is_none()
    );
    assert!(
        state
            .get_json::<serde_json::Value>("usage:module:demo")?
            .is_some()
    );
    assert_eq!(
        state
            .connection
            .query_row("SELECT COUNT(*) FROM usage_outbox", [], |row| row
                .get::<_, i64>(0))?,
        1
    );
    assert_eq!(state.pending_usage_count()?, 0);
    Ok(())
}

#[tokio::test]
async fn interrupted_credential_cleanup_resumes_without_loading_a_deleted_key() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.request()?;
    fixture.retirement.prepare().await?;
    let outside = fixture.root.join("outside");
    fs::write(&outside, "untouched")?;
    fs::remove_file(fixture.config.identity_dir.join("server_id"))?;
    symlink(&outside, fixture.config.identity_dir.join("server_id"))?;
    assert!(
        fixture
            .retirement
            .complete(&fixture.identity)
            .await
            .is_err()
    );
    assert!(!fixture.config.identity_dir.join("device.key").exists());
    assert_eq!(fixture.retirement.read()?.unwrap().phase, Phase::Clearing);
    assert_eq!(fs::read_to_string(&outside)?, "untouched");
    fs::remove_file(fixture.config.identity_dir.join("server_id"))?;
    let recovered = Retirement::new(
        fixture.config.clone(),
        fixture.state.clone(),
        vec![Arc::new(FakeAdapter::default())],
        Arc::new(SystemOps),
        fixture.services.clone(),
    )?;
    // The panel is unavailable, but credential cleanup must complete before a receipt retry.
    assert!(recovered.recover_completion().await.is_err());
    assert_eq!(recovered.read()?.unwrap().phase, Phase::Completed);
    assert!(!fixture.config.identity_dir.join("panel_origin").exists());
    assert_eq!(fs::read_to_string(outside)?, "untouched");
    Ok(())
}

#[tokio::test]
async fn managed_directory_alias_is_rejected_before_any_credential_deletion() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.request()?;
    let original = fixture.config.runtime_root.join("demo@main");
    let outside = fixture.root.join("outside-runtime");
    fs::rename(&original, &outside)?;
    symlink(&outside, &original)?;
    assert!(fixture.retirement.prepare().await.is_err());
    assert!(outside.join("revisions/1/config.json").exists());
    assert!(fixture.config.identity_dir.join("device.key").exists());
    Ok(())
}

struct DiagnosticServices {
    runtime: FakeServiceManager,
    job_running: AtomicBool,
    fail_stop: AtomicBool,
}
impl ServiceManager for DiagnosticServices {
    fn reload<'a>(&'a self, unit: &'a str) -> sinan_adapter_sdk::BoxFuture<'a, ()> {
        self.runtime.reload(unit)
    }
    fn restart<'a>(&'a self, unit: &'a str) -> sinan_adapter_sdk::BoxFuture<'a, ()> {
        self.runtime.restart(unit)
    }
    fn is_active<'a>(&'a self, unit: &'a str) -> sinan_adapter_sdk::BoxFuture<'a, bool> {
        self.runtime.is_active(unit)
    }
    fn stop<'a>(&'a self, unit: &'a str) -> sinan_adapter_sdk::BoxFuture<'a, ()> {
        Box::pin(async move {
            if unit.starts_with("sinan-diagnostic-") {
                ensure!(
                    !self.fail_stop.swap(false, Ordering::SeqCst),
                    "injected diagnostic stop failure"
                );
                self.job_running.store(false, Ordering::SeqCst);
                Ok(())
            } else {
                self.runtime.stop(unit).await
            }
        })
    }
    fn job_status<'a>(
        &'a self,
        _: &'a str,
    ) -> sinan_adapter_sdk::BoxFuture<'a, sinan_adapter_sdk::JobStatus> {
        Box::pin(async move {
            Ok(if self.job_running.load(Ordering::SeqCst) {
                sinan_adapter_sdk::JobStatus::Running
            } else {
                sinan_adapter_sdk::JobStatus::Missing
            })
        })
    }
}

#[tokio::test]
async fn diagnostic_shutdown_failure_prevents_success_and_can_be_retried() -> Result<()> {
    let fixture = Fixture::new()?;
    let job_id = Uuid::new_v4();
    let spec = sinan_adapter_sdk::DiagnosticSpec {
        id: job_id.to_string(),
        version: "test".into(),
        binary_path: fixture.config.install_root.join("diagnostic/test/tool"),
        job_dir: fixture
            .config
            .runtime_root
            .join("diagnostics")
            .join(job_id.to_string()),
        timeout_secs: 60,
        options: BTreeMap::new(),
    };
    let service = sinan_adapter_sdk::ServiceJob {
        unit: format!("sinan-diagnostic-{job_id}.service"),
        program: spec.binary_path.clone(),
        args: vec![],
        working_directory: spec.job_dir.clone(),
        timeout_secs: 60,
    };
    fixture.state.lock().unwrap().set_json("diagnostics:active", &serde_json::json!({
        "Started": { "spec": spec, "service": service, "started_at": 0, "plugin": "test", "start_error": null, "expires_at": null }
    }))?;
    let services = Arc::new(DiagnosticServices {
        runtime: FakeServiceManager::default(),
        job_running: AtomicBool::new(true),
        fail_stop: AtomicBool::new(true),
    });
    let retirement = Retirement::new(
        fixture.config.clone(),
        fixture.state.clone(),
        vec![Arc::new(FakeAdapter::default())],
        Arc::new(SystemOps),
        services.clone(),
    )?;
    retirement.request(
        &fixture.identity,
        RetirementRequest {
            request_id: Uuid::new_v4(),
        },
    )?;
    assert!(retirement.prepare().await.is_err());
    assert!(services.job_running.load(Ordering::SeqCst));
    assert!(fixture.config.identity_dir.join("device.key").exists());
    retirement.prepare().await?;
    assert!(!services.job_running.load(Ordering::SeqCst));
    retirement.complete(&fixture.identity).await?;
    assert!(!fixture.config.identity_dir.join("device.key").exists());
    Ok(())
}

#[tokio::test]
async fn runtime_root_alias_cannot_remove_an_outside_current_link() -> Result<()> {
    let fixture = Fixture::new()?;
    let alias = fixture.root.join("runtime-alias");
    symlink(&fixture.config.runtime_root, &alias)?;
    let mut config = fixture.config.clone();
    config.runtime_root = alias;
    let retirement = Retirement::new(
        config,
        fixture.state.clone(),
        vec![Arc::new(FakeAdapter::default())],
        Arc::new(SystemOps),
        fixture.services.clone(),
    )?;
    retirement.request(
        &fixture.identity,
        RetirementRequest {
            request_id: Uuid::new_v4(),
        },
    )?;
    assert!(retirement.prepare().await.is_err());
    assert!(
        fixture
            .config
            .runtime_root
            .join("demo@main/current")
            .exists()
    );
    assert!(fixture.config.identity_dir.join("device.key").exists());
    Ok(())
}

struct ControlledSystemctl {
    query_fails: AtomicBool,
    running: AtomicBool,
}

impl Privileged for ControlledSystemctl {
    fn execute<'a>(
        &'a self,
        program: &'a Path,
        args: &'a [String],
    ) -> BoxFuture<'a, CommandOutput> {
        Box::pin(async move {
            ensure!(program == Path::new("systemctl"), "unexpected program");
            match args.first().map(String::as_str) {
                Some("show") if self.query_fails.load(Ordering::SeqCst) => Ok(CommandOutput {
                    success: false,
                    stdout: String::new(),
                    stderr: "Failed to connect to bus: Permission denied".into(),
                }),
                Some("show") => {
                    let (state, pid) = if self.running.load(Ordering::SeqCst) {
                        ("active", 123)
                    } else {
                        ("inactive", 0)
                    };
                    Ok(CommandOutput {
                        success: true,
                        stdout: format!(
                            "LoadState=loaded\nActiveState={state}\nMainPID={pid}\nControlPID=0\n"
                        ),
                        stderr: String::new(),
                    })
                }
                Some("stop") => {
                    self.running.store(false, Ordering::SeqCst);
                    Ok(CommandOutput {
                        success: true,
                        ..Default::default()
                    })
                }
                // Reproduces the previous is-active behavior on a bus failure.
                Some("is-active") => Ok(CommandOutput {
                    success: false,
                    stdout: String::new(),
                    stderr: "Failed to connect to bus: Permission denied".into(),
                }),
                _ => anyhow::bail!("unexpected systemctl action"),
            }
        })
    }
    fn create_dir<'a>(&'a self, _: &'a Path, _: u32, _: Option<&'a str>) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected create_dir") })
    }
    fn write_file<'a>(
        &'a self,
        _: &'a Path,
        _: &'a [u8],
        _: u32,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected write_file") })
    }
    fn atomic_symlink<'a>(&'a self, _: &'a Path, _: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected atomic_symlink") })
    }
    fn remove_symlink<'a>(&'a self, _: &'a Path) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected remove_symlink") })
    }
    fn install_archive<'a>(&'a self, _: &'a Path, _: &'a Path, _: &'a str) -> BoxFuture<'a, ()> {
        Box::pin(async { anyhow::bail!("unexpected install_archive") })
    }
}

#[tokio::test]
async fn systemd_query_failure_cannot_clear_credentials_or_confirm_retirement() -> Result<()> {
    let fixture = Fixture::new()?;
    fixture.request()?;
    let systemctl = Arc::new(ControlledSystemctl {
        query_fails: AtomicBool::new(true),
        running: AtomicBool::new(true),
    });
    let retirement = Retirement::new(
        fixture.config.clone(),
        fixture.state.clone(),
        vec![Arc::new(FakeAdapter::default())],
        Arc::new(SystemOps),
        Arc::new(SystemServiceManager::new(systemctl.clone())),
    )?;
    assert!(retirement.prepare().await.is_err());
    assert!(systemctl.running.load(Ordering::SeqCst));
    assert_eq!(retirement.read()?.unwrap().phase, Phase::Requested);
    assert!(retirement.complete(&fixture.identity).await.is_err());
    assert!(retirement.deliver_receipt().await.is_err());
    assert!(fixture.config.identity_dir.join("device.key").exists());
    assert!(
        fixture
            .config
            .runtime_root
            .join("demo@main/current")
            .exists()
    );

    systemctl.query_fails.store(false, Ordering::SeqCst);
    retirement.prepare().await?;
    assert!(!systemctl.running.load(Ordering::SeqCst));
    assert_eq!(retirement.read()?.unwrap().phase, Phase::Stopped);

    // A status failure during the final check must also preserve credentials,
    // even though the public receipt has already been saved for crash recovery.
    systemctl.running.store(true, Ordering::SeqCst);
    systemctl.query_fails.store(true, Ordering::SeqCst);
    assert!(retirement.complete(&fixture.identity).await.is_err());
    assert_eq!(retirement.read()?.unwrap().phase, Phase::Clearing);
    assert!(retirement.deliver_receipt().await.is_err());
    assert!(retirement.recover_completion().await.is_err());
    assert!(systemctl.running.load(Ordering::SeqCst));
    for name in ["device.key", "server_id", "panel_origin"] {
        assert!(fixture.config.identity_dir.join(name).exists());
    }
    assert!(
        fixture
            .config
            .runtime_root
            .join("demo@main/revisions/1/config.json")
            .exists()
    );
    assert!(ensure_enrollment_allowed(&fixture.config).is_err());
    Ok(())
}
