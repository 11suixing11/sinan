use super::*;
use crate::system::deploy::reference;
use sinan_adapter_sdk::ManagedProcess;

async fn save(root: &Path, state: &UpgradeState, ops: &dyn Privileged) -> Result<()> {
    ops.write_file(
        &root.join("update-state.json"),
        &serde_json::to_vec_pretty(state)?,
        0o600,
        None,
    )
    .await
}
async fn spawn(
    root: &Path,
    version: &str,
    path: &Path,
    monitor_only: bool,
    ops: &dyn Privileged,
) -> Result<Box<dyn ManagedProcess>> {
    ensure!(
        release_version(version).is_some(),
        "invalid installed Agent version"
    );
    let mut args = vec![
        "--config".into(),
        path.to_string_lossy().into_owned(),
        "run".into(),
    ];
    if monitor_only {
        args.push("--monitor-only".into());
    }
    ops.spawn_managed(&root.join(version).join(executable_name()), &args)
        .await
}
async fn ready(process: &mut dyn ManagedProcess, config: &Config, version: &str) -> Result<()> {
    let started = Instant::now();
    let mut consecutive = 0;
    while started.elapsed() < Duration::from_secs(60) {
        ensure!(
            process.try_wait()?.is_none(),
            "new Agent exited during startup"
        );
        if let Ok(status) = crate::transport::status(&config.status_socket).await
            && status["agent_version"] == version
            && status["pid"].as_u64() == Some(u64::from(process.id()))
        {
            consecutive += 1;
            if consecutive >= 3 {
                return Ok(());
            }
        } else {
            consecutive = 0;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    anyhow::bail!("new Agent did not become healthy within 60 seconds")
}

pub async fn supervise(
    config: Config,
    path: std::path::PathBuf,
    monitor_only: bool,
    ops: Arc<dyn Privileged>,
) -> Result<()> {
    #[cfg(unix)]
    let shutdown = async {
        let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        let mut interrupt =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
        tokio::select! {_=term.recv()=>{},_=interrupt.recv()=>{}}
        Ok::<_, anyhow::Error>(())
    };
    #[cfg(windows)]
    let shutdown = async {
        tokio::signal::ctrl_c().await?;
        Ok::<_, anyhow::Error>(())
    };
    tokio::select! { result=supervise_inner(config,path,monitor_only,ops)=>result, result=shutdown=>result }
}

async fn supervise_inner(
    config: Config,
    path: std::path::PathBuf,
    monitor_only: bool,
    ops: Arc<dyn Privileged>,
) -> Result<()> {
    let root = core_root(&config)?;
    ops.create_dir(
        config.state_db.parent().context("state has no parent")?,
        0o700,
        None,
    )
    .await?;
    let lock = rusqlite::Connection::open(
        config
            .state_db
            .parent()
            .context("state has no parent")?
            .join("supervisor.lock"),
    )?;
    lock.busy_timeout(Duration::ZERO)?;
    lock.execute_batch("BEGIN EXCLUSIVE")?;
    let current = reference(&root.join("current"))?;
    ensure!(
        current.parent() == Some(root.as_path()),
        "Agent reference must point to an installed release"
    );
    let mut state: UpgradeState = if root.join("update-state.json").try_exists()? {
        read_json(&root.join("update-state.json"))?
    } else {
        UpgradeState::default()
    };
    let current = current
        .file_name()
        .and_then(|v| v.to_str())
        .context("Agent release path is invalid")?;
    if let Some(trial) = state.trial.take() {
        state.failed(trial.version);
        state.current = state
            .previous
            .clone()
            .context("interrupted upgrade has no previous release")?;
        state.last_error = Some(
            "supervisor restarted during an unconfirmed update; restored previous release".into(),
        );
        ops.atomic_symlink(&root.join("current"), &root.join(&state.current))
            .await?;
    } else {
        state.current = current.into();
    }
    save(&root, &state, ops.as_ref()).await?;
    let mut child = spawn(&root, &state.current, &path, monitor_only, ops.as_ref()).await?;
    let mut crashes = 0u32;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if child.try_wait()?.is_some() {
            crashes = crashes.saturating_add(1);
            tokio::time::sleep(Duration::from_secs((1u64 << crashes.min(5)).min(30))).await;
            child = spawn(&root, &state.current, &path, monitor_only, ops.as_ref()).await?;
        }
        let pending: Option<PendingUpgrade> = if root.join("pending-update.json").try_exists()? {
            read_json(&root.join("pending-update.json"))?
        } else {
            None
        };
        let Some(pending) = pending else {
            continue;
        };
        if state.failed_versions.contains(&pending.version)
            || release_version(&pending.version) <= release_version(&state.current)
        {
            ops.write_file(&root.join("pending-update.json"), b"null", 0o600, None)
                .await?;
            continue;
        }
        let validation = verify(
            &root.join(&pending.version).join(executable_name()),
            &pending.version,
            &pending.sha256,
            ops.as_ref(),
        )
        .await;
        if let Err(error) = validation {
            state.failed(pending.version);
            state.last_error = Some(error.to_string());
            save(&root, &state, ops.as_ref()).await?;
            ops.write_file(&root.join("pending-update.json"), b"null", 0o600, None)
                .await?;
            continue;
        }
        state.previous = Some(state.current.clone());
        state.trial = Some(pending.clone());
        save(&root, &state, ops.as_ref()).await?;
        child.terminate().await?;
        let attempt = async {
            let mut candidate =
                spawn(&root, &pending.version, &path, monitor_only, ops.as_ref()).await?;
            if let Err(error) = ready(candidate.as_mut(), &config, &pending.version).await {
                let _ = candidate.terminate().await;
                return Err(error);
            }
            Ok::<_, anyhow::Error>(candidate)
        }
        .await;
        match attempt {
            Ok(candidate) => {
                child = candidate;
                ops.atomic_symlink(&root.join("current"), &root.join(&pending.version))
                    .await?;
                state.current = pending.version;
                state.last_error = None;
                crashes = 0;
            }
            Err(error) => {
                state.failed(pending.version);
                state.last_error = Some(error.to_string());
                child = spawn(&root, &state.current, &path, monitor_only, ops.as_ref()).await?;
            }
        }
        state.trial = None;
        save(&root, &state, ops.as_ref()).await?;
        ops.write_file(&root.join("pending-update.json"), b"null", 0o600, None)
            .await?;
    }
}
