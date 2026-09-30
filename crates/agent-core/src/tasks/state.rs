use crate::State;
use anyhow::{Result, ensure};
use rusqlite::{OptionalExtension, params};
use sinan_protocol::{
    CommandResult, CommandStatus, ProbeResult, RemoteCommand, now_timestamp, telemetry::now_millis,
};
use uuid::Uuid;

impl State {
    pub fn begin_command(&mut self, command: &RemoteCommand) -> Result<bool> {
        ensure!(command.valid(), "invalid remote command");
        let previous: Option<String> = self
            .connection
            .query_row(
                "SELECT spec FROM command_journal WHERE id=?1",
                [command.id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(previous) = previous {
            ensure!(
                serde_json::from_str::<RemoteCommand>(&previous)? == *command,
                "command identifier changed content"
            );
            return Ok(false);
        }
        self.connection.execute("DELETE FROM command_journal WHERE acknowledged=1 AND CAST(json_extract(spec,'$.expires_at') AS INTEGER)<?1", [now_timestamp()-7*86400])?;
        let count: i64 =
            self.connection
                .query_row("SELECT COUNT(*) FROM command_journal", [], |r| r.get(0))?;
        ensure!(count < 10_000, "command journal reached retention limit");
        self.connection.execute(
            "INSERT INTO command_journal(id,spec) VALUES(?1,?2)",
            params![command.id.to_string(), serde_json::to_string(command)?],
        )?;
        Ok(true)
    }

    pub fn finish_command(&mut self, result: &CommandResult) -> Result<()> {
        let updated = self.connection.execute(
            "UPDATE command_journal SET result=?2 WHERE id=?1 AND result IS NULL",
            params![result.id.to_string(), serde_json::to_string(result)?],
        )?;
        ensure!(updated == 1, "command was not running or already finished");
        Ok(())
    }

    pub fn recover_commands(&mut self) -> Result<()> {
        let ids = {
            let mut query = self
                .connection
                .prepare("SELECT id FROM command_journal WHERE result IS NULL")?;
            query
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?
        };
        for id in ids {
            self.finish_command(&CommandResult {
                id: Uuid::parse_str(&id)?,
                status: CommandStatus::Interrupted,
                finished_at: now_timestamp()
                    + self.get_json::<i64>("clock_offset_ms")?.unwrap_or(0) / 1000,
                stdout: String::new(),
                stderr: "Agent restarted; command was not executed again".into(),
                timed_out: false,
                truncated: false,
            })?;
        }
        Ok(())
    }

    pub fn command_results(&self) -> Result<Vec<CommandResult>> {
        let mut query = self.connection.prepare("SELECT result FROM command_journal WHERE result IS NOT NULL AND acknowledged=0 ORDER BY rowid LIMIT 64")?;
        let rows = query
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .map(|v| serde_json::from_str(&v))
            .collect::<Result<_, _>>()?)
    }

    pub fn acknowledge_command(&mut self, id: Uuid) -> Result<()> {
        self.connection.execute(
            "UPDATE command_journal SET acknowledged=1 WHERE id=?1 AND result IS NOT NULL",
            [id.to_string()],
        )?;
        Ok(())
    }

    pub fn save_probe_result(&mut self, result: &ProbeResult) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute(
            "INSERT INTO probe_outbox(id,sampled_at,result) VALUES(?1,?2,?3)",
            params![
                result.id.to_string(),
                result.sampled_at,
                serde_json::to_string(result)?
            ],
        )?;
        tx.execute(
            "DELETE FROM probe_outbox WHERE sampled_at<?1",
            [now_millis() - 2 * 3_600_000],
        )?;
        tx.execute("DELETE FROM probe_outbox WHERE id IN (SELECT id FROM probe_outbox ORDER BY sampled_at DESC,rowid DESC LIMIT -1 OFFSET 4096)", [])?;
        tx.commit()?;
        Ok(())
    }

    pub fn probe_results(&self) -> Result<Vec<ProbeResult>> {
        let mut query = self
            .connection
            .prepare("SELECT result FROM probe_outbox ORDER BY sampled_at,rowid LIMIT 64")?;
        let rows = query
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .map(|v| serde_json::from_str(&v))
            .collect::<Result<_, _>>()?)
    }

    pub fn acknowledge_probes(&mut self, ids: &[Uuid]) -> Result<()> {
        ensure!(ids.len() <= 64, "too many probe acknowledgments");
        let tx = self.connection.transaction()?;
        for id in ids {
            tx.execute("DELETE FROM probe_outbox WHERE id=?1", [id.to_string()])?;
        }
        tx.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
