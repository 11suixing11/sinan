use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteCommand {
    pub id: Uuid,
    pub command: String,
    pub timeout_secs: u32,
    pub expires_at: i64,
}

impl RemoteCommand {
    pub fn valid(&self) -> bool {
        !self.command.trim().is_empty()
            && self.command.len() <= 16_384
            && !self.command.contains('\0')
            && (1..=600).contains(&self.timeout_secs)
            && self.expires_at > 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Cancelled,
    Succeeded,
    Failed,
    Expired,
    Interrupted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandState {
    Queued,
    Claimed,
    Running,
    CancelRequested,
    Cancelled,
    Succeeded,
    Failed,
    Expired,
    Interrupted,
}

impl CommandState {
    pub fn terminal(self) -> bool {
        matches!(
            self,
            Self::Cancelled | Self::Succeeded | Self::Failed | Self::Expired | Self::Interrupted
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandClaim {
    pub claim_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandStarted {
    pub claim_id: Uuid,
    pub started_at: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommandControl {
    pub id: Uuid,
    pub state: CommandState,
    pub claimed_at: Option<i64>,
    pub started_at: Option<i64>,
    pub cancel_requested_at: Option<i64>,
    pub finished_at: Option<i64>,
    pub cancel_supported: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandResult {
    pub id: Uuid,
    pub status: CommandStatus,
    pub finished_at: i64,
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
    pub truncated: bool,
}

mod probes;
pub use probes::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeBatch {
    pub results: Vec<ProbeResult>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskAck {
    pub ids: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_results_and_commands_remain_wire_compatible() {
        let command = serde_json::json!({"id":Uuid::nil(),"command":"echo fixture","timeout_secs":3,"expires_at":123});
        assert!(
            serde_json::from_value::<RemoteCommand>(command)
                .unwrap()
                .valid()
        );
        let result = serde_json::json!({"id":Uuid::nil(),"status":"succeeded","finished_at":123,"stdout":"","stderr":"","timed_out":false,"truncated":false});
        let decoded: CommandResult = serde_json::from_value(result.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), result);
        let mut running = result;
        running["status"] = serde_json::json!("running");
        assert!(serde_json::from_value::<CommandResult>(running).is_err());
    }

    #[test]
    fn cancellation_request_is_not_a_terminal_result() {
        assert!(!CommandState::CancelRequested.terminal());
        assert!(!CommandState::Claimed.terminal());
        assert!(CommandState::Cancelled.terminal());
        assert_eq!(
            serde_json::to_string(&CommandStatus::Cancelled).unwrap(),
            "\"cancelled\""
        );
    }
}
