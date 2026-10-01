use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const RUNTIME_VALIDATION_CAPABILITY: &str = "runtime:dependency-validation:v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeValidationOperation {
    Probe,
    Barrier,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeValidationRequest {
    pub id: Uuid,
    pub module: String,
    pub scope: String,
    pub generation: u64,
    pub operation: RuntimeValidationOperation,
    pub revision: u64,
    pub config_hash: String,
    pub expires_at: i64,
}

pub fn valid_runtime_scope(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
}

impl RuntimeValidationRequest {
    pub fn valid(&self) -> bool {
        valid_runtime_scope(&self.module)
            && self.module.len() <= 64
            && valid_runtime_scope(&self.scope)
            && self.generation > 0
            && self.generation <= i64::MAX as u64
            && self.revision > 0
            && self.revision <= i64::MAX as u64
            && self.config_hash.len() == 64
            && self
                .config_hash
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            && self.expires_at > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeValidationError {
    Expired,
    Interrupted,
    ModuleUnavailable,
    RevisionChanged,
    InvalidConstraints,
    UntrustedArtifact,
    ValidationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeValidationResult {
    #[serde(flatten)]
    pub request: RuntimeValidationRequest,
    pub success: bool,
    pub error: Option<RuntimeValidationError>,
    pub checked_at: i64,
}

impl RuntimeValidationResult {
    pub fn valid(&self) -> bool {
        self.request.valid() && self.checked_at > 0 && self.success == self.error.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation_identity_is_exact_and_result_codes_are_closed() {
        let request = RuntimeValidationRequest {
            id: Uuid::new_v4(),
            module: "demo".into(),
            scope: "scope:1".into(),
            generation: 2,
            operation: RuntimeValidationOperation::Probe,
            revision: 3,
            config_hash: "a".repeat(64),
            expires_at: 100,
        };
        assert!(request.valid());
        let result = RuntimeValidationResult {
            request,
            success: true,
            error: None,
            checked_at: 99,
        };
        assert!(result.valid());
        let mut wire = serde_json::to_value(&result).unwrap();
        assert_eq!(
            serde_json::from_value::<RuntimeValidationResult>(wire.clone()).unwrap(),
            result
        );
        wire["error"] = "secret native error".into();
        assert!(serde_json::from_value::<RuntimeValidationResult>(wire).is_err());
    }
}
