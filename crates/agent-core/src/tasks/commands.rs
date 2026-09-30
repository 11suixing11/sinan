use crate::{SharedState, artifacts::PanelClient};
use anyhow::{Result, ensure};
use sinan_adapter_sdk::Privileged;
use sinan_protocol::{CommandResult, CommandStatus, RemoteCommand, TaskAck, now_timestamp};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::watch;

pub(super) async fn run(
    allow_remote_commands: bool,
    state: SharedState,
    ops: Arc<dyn Privileged>,
    clients: watch::Receiver<Option<Arc<PanelClient>>>,
    retirement: Arc<crate::retirement::Retirement>,
) -> Result<()> {
    // This value comes only from local Agent configuration, never panel settings.
    // Do not even fetch queued commands when the operator has not opted in.
    if !allow_remote_commands {
        return Ok(());
    }
    {
        let _guard = retirement.gate.read().await;
        if !retirement.requested() {
            state
                .lock()
                .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
                .recover_commands()?;
        }
    }
    loop {
        let client = clients.borrow().clone();
        if let Some(client) = client
            && let Err(error) = tick(&state, ops.as_ref(), &client, &retirement).await
        {
            tracing::warn!(%error,"command worker will retry durable results");
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

async fn tick(
    state: &SharedState,
    ops: &dyn Privileged,
    client: &PanelClient,
    retirement: &crate::retirement::Retirement,
) -> Result<()> {
    let _guard = retirement.gate.read().await;
    if retirement.requested() {
        return Ok(());
    }
    let results = state
        .lock()
        .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
        .command_results()?;
    for result in results {
        if retirement.requested() {
            return Ok(());
        }
        let ack: TaskAck = client
            .post_json(&format!("/api/agent/v1/commands/{}", result.id), &result)
            .await?;
        ensure!(ack.ids == [result.id], "invalid command acknowledgment");
        state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .acknowledge_command(result.id)?;
    }
    if retirement.requested() {
        return Ok(());
    }
    let commands: Vec<RemoteCommand> = client.get_json("/api/agent/v1/commands").await?;
    ensure!(
        commands.len() <= 64 && commands.iter().all(RemoteCommand::valid),
        "invalid commands from panel"
    );
    for command in commands {
        if retirement.requested() {
            break;
        }
        let first = state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .begin_command(&command)?;
        if !first {
            continue;
        }
        let offset = state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .get_json::<i64>("clock_offset_ms")?
            .unwrap_or(0)
            / 1000;
        let result = execute(ops, &command, offset).await;
        state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .finish_command(&result)?;
        let ack: TaskAck = client
            .post_json(&format!("/api/agent/v1/commands/{}", result.id), &result)
            .await?;
        ensure!(ack.ids == [result.id], "invalid command acknowledgment");
        state
            .lock()
            .map_err(|_| anyhow::anyhow!("state lock poisoned"))?
            .acknowledge_command(result.id)?;
    }
    Ok(())
}

pub(super) async fn execute(
    ops: &dyn Privileged,
    command: &RemoteCommand,
    clock_offset: i64,
) -> CommandResult {
    let mut result = CommandResult {
        id: command.id,
        status: CommandStatus::Expired,
        finished_at: now_timestamp() + clock_offset,
        stdout: String::new(),
        stderr: String::new(),
        timed_out: false,
        truncated: false,
    };
    if command.expires_at <= now_timestamp() + clock_offset {
        return result;
    }
    #[cfg(unix)]
    let (program, args) = (
        Path::new("/bin/sh"),
        vec!["-c".into(), command.command.clone()],
    );
    #[cfg(windows)]
    let (program, args) = (
        Path::new("powershell.exe"),
        vec![
            "-NoProfile".into(),
            "-NonInteractive".into(),
            "-Command".into(),
            command.command.clone(),
        ],
    );
    match ops
        .execute_bounded(program, &args, command.timeout_secs, 256 * 1024)
        .await
    {
        Ok(output) => {
            result.status = if output.output.success && !output.timed_out {
                CommandStatus::Succeeded
            } else {
                CommandStatus::Failed
            };
            result.stdout = output.output.stdout;
            result.stderr = output.output.stderr;
            result.timed_out = output.timed_out;
            result.truncated = output.truncated;
        }
        Err(error) => {
            result.status = CommandStatus::Failed;
            result.stderr = error.to_string().chars().take(1024).collect();
        }
    }
    result.finished_at = now_timestamp() + clock_offset;
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Config, State, fake::FakeServiceManager, retirement::Retirement, system::SystemOps,
    };
    use std::sync::Mutex;
    use uuid::Uuid;

    #[tokio::test]
    async fn disabled_commands_do_not_contact_panel_or_touch_pending_execution() -> Result<()> {
        let directory =
            std::env::temp_dir().join(format!("sinan-command-opt-in-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory)?;
        let root = std::fs::canonicalize(&directory)?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let config = Config {
            panel_url: format!("http://{}", listener.local_addr()?),
            state_db: root.join("state.db"),
            runtime_root: root.join("runtime"),
            install_root: root.join("install"),
            ..Config::default()
        };
        let state = Arc::new(Mutex::new(State::open(&config.state_db)?));
        let command = RemoteCommand {
            id: Uuid::new_v4(),
            command: "echo should-not-execute".into(),
            timeout_secs: 1,
            expires_at: now_timestamp() + 60,
        };
        state.lock().unwrap().begin_command(&command)?;
        let ops = Arc::new(SystemOps);
        let retirement = Arc::new(Retirement::new(
            config.clone(),
            state.clone(),
            Vec::new(),
            ops.clone(),
            Arc::new(FakeServiceManager::default()),
        )?);
        let (_sender, receiver) = watch::channel(Some(Arc::new(PanelClient::new(
            &config.panel_url,
            "TEST_ONLY_session",
        )?)));
        tokio::time::timeout(
            Duration::from_secs(1),
            run(
                config.allow_remote_commands,
                state.clone(),
                ops,
                receiver,
                retirement,
            ),
        )
        .await??;
        assert!(
            tokio::time::timeout(Duration::from_millis(30), listener.accept())
                .await
                .is_err()
        );
        // Even recovery of earlier work is untouched while the local permission is off.
        assert!(state.lock().unwrap().command_results()?.is_empty());
        assert!(!state.lock().unwrap().begin_command(&command)?);
        drop(state);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }
}
