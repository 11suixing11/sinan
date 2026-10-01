use super::*;
#[test]
fn command_finish_time_never_precedes_the_durable_start_after_clock_adjustment() -> Result<()> {
    let mut state = State::open(std::path::Path::new(":memory:"))?;
    let command = RemoteCommand {
        id: Uuid::new_v4(),
        command: "true".into(),
        timeout_secs: 1,
        expires_at: now_timestamp() + 30,
    };
    state.queue_command(&command)?;
    let record = state.pending_commands()?.remove(0);
    let started = now_timestamp() + 1;
    state.command_spawned(
        command.id,
        &CommandProcessIdentity {
            pid: 12345,
            started: "TEST_ONLY_process".into(),
        },
    )?;
    state.command_started(
        command.id,
        &CommandStarted {
            claim_id: record.claim_id,
            started_at: started,
        },
    )?;
    state.finish_command(&CommandResult {
        id: command.id,
        status: CommandStatus::Succeeded,
        finished_at: started - 1,
        stdout: String::new(),
        stderr: String::new(),
        timed_out: false,
        truncated: false,
    })?;
    assert_eq!(state.command_results()?[0].finished_at, started);
    Ok(())
}
#[test]
fn lifecycle_claim_start_and_cancellation_survive_reopen_without_reexecution() -> Result<()> {
    let path = std::env::temp_dir().join(format!("sn-command-lifecycle-{}.db", Uuid::new_v4()));
    let command = RemoteCommand {
        id: Uuid::new_v4(),
        command: "echo fixture".into(),
        timeout_secs: 5,
        expires_at: now_timestamp() + 30,
    };
    let claim;
    let started;
    {
        let mut state = State::open(&path)?;
        state.queue_command(&command)?;
        state.queue_command(&command)?;
        let records = state.pending_commands()?;
        assert_eq!(records.len(), 1);
        claim = records[0].claim_id;
        assert!(!records[0].recovering);
        state.command_spawned(
            command.id,
            &CommandProcessIdentity {
                pid: 12345,
                started: "TEST_ONLY_identity".into(),
            },
        )?;
        started = CommandStarted {
            claim_id: claim,
            started_at: now_timestamp(),
        };
        state.command_started(command.id, &started)?;
        state.request_command_cancel(command.id)?;
    }
    let mut state = State::open(&path)?;
    state.recover_commands()?;
    let records = state.pending_commands()?;
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].claim_id, claim);
    assert!(records[0].recovering && records[0].cancel_requested);
    assert!(records[0].process.is_some());
    assert_eq!(state.command_starts()?, vec![(command.id, started.clone())]);
    assert!(
        state
            .command_spawned(
                command.id,
                &CommandProcessIdentity {
                    pid: 12345,
                    started: "TEST_ONLY_identity".into()
                }
            )
            .is_err()
    );
    let result = CommandResult {
        id: command.id,
        status: CommandStatus::Cancelled,
        finished_at: now_timestamp(),
        stdout: String::new(),
        stderr: String::new(),
        timed_out: false,
        truncated: false,
    };
    state.finish_command(&result)?;
    state.acknowledge_command(command.id)?;
    // Result acknowledgment cannot discard a delayed durable start report.
    assert_eq!(state.command_starts()?, vec![(command.id, started)]);
    state.acknowledge_command_start(command.id)?;
    assert!(state.command_starts()?.is_empty());
    assert!(state.pending_commands()?.is_empty());
    state.queue_command(&command)?;
    assert!(state.pending_commands()?.is_empty());
    let mut changed = command;
    changed.command = "echo changed".into();
    assert!(state.queue_command(&changed).is_err());
    drop(state);
    std::fs::remove_file(path)?;
    Ok(())
}
#[test]
fn probes_survive_restart_and_partial_ack_with_panel_clock_offset() -> Result<()> {
    let path = std::env::temp_dir().join(format!("sn-probes-{}.db", Uuid::new_v4()));
    let offset = -6 * 3_600_000;
    let result = ProbeResult {
        id: Uuid::new_v4(),
        probe_id: Uuid::new_v4(),
        sampled_at: now_millis() + offset,
        latency_ms: None,
        loss_percent: 100.0,
        error: Some("ICMP tool unavailable".into()),
    };
    let next = ProbeResult {
        id: Uuid::new_v4(),
        latency_ms: Some(0.0),
        loss_percent: 0.0,
        error: None,
        ..result.clone()
    };
    {
        let mut state = State::open(&path)?;
        state.set_json("clock_offset_ms", &offset)?;
        state.save_probe_result(&result)?;
        state.save_probe_result(&next)?;
        assert_eq!(state.probe_results()?, vec![result.clone(), next.clone()]);
    }
    let mut state = State::open(&path)?;
    assert_eq!(state.probe_results()?, vec![result.clone(), next.clone()]);
    state.acknowledge_probes(&[result.id])?;
    assert_eq!(state.probe_results()?, vec![next.clone()]);
    state.acknowledge_probes(&[result.id, next.id])?;
    assert!(state.probe_results()?.is_empty());
    let expired = ProbeResult {
        id: Uuid::new_v4(),
        sampled_at: now_millis() + offset - 3 * 3_600_000,
        ..result
    };
    state.save_probe_result(&expired)?;
    assert!(state.probe_results()?.is_empty());
    drop(state);
    std::fs::remove_file(path)?;
    Ok(())
}
#[test]
fn command_recovery_never_reexecutes_and_results_survive_ack_loss() -> Result<()> {
    let path = std::env::temp_dir().join(format!("sn-command-{}.db", Uuid::new_v4()));
    let command = RemoteCommand {
        id: Uuid::new_v4(),
        command: "echo test".into(),
        timeout_secs: 3,
        expires_at: now_timestamp() + 30,
    };
    {
        let mut state = State::open(&path)?;
        assert!(state.begin_command(&command)?);
    }
    let mut state = State::open(&path)?;
    state.recover_commands()?;
    assert!(!state.begin_command(&command)?);
    let results = state.command_results()?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].status, CommandStatus::Interrupted);
    assert!(state.finish_command(&results[0]).is_err());
    state.acknowledge_command(command.id)?;
    assert!(state.command_results()?.is_empty());
    assert!(!state.begin_command(&command)?);
    drop(state);
    std::fs::remove_file(path)?;
    Ok(())
}
