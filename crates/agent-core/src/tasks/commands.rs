use crate::{SharedState, artifacts::PanelClient};
use anyhow::{Result, ensure};
use sinan_adapter_sdk::Privileged;
use sinan_protocol::{CommandResult, CommandStatus, RemoteCommand, TaskAck, now_timestamp};
use std::{path::Path, sync::Arc, time::Duration};
use tokio::sync::watch;

pub(super) async fn run(
    state: SharedState,
    ops: Arc<dyn Privileged>,
    clients: watch::Receiver<Option<Arc<PanelClient>>>,
    retirement: Arc<crate::retirement::Retirement>,
) -> Result<()> {
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
