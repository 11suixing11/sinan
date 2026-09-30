use crate::Artifact;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

/// A fixed, registered diagnostic plugin task; never an arbitrary shell command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticJob {
    pub id: Uuid,
    pub plugin: String,
    pub version: String,
    pub artifact: Artifact,
    pub timeout_secs: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticReport {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticUpdate {
    pub id: Uuid,
    pub status: DiagnosticStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report: Option<DiagnosticReport>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Chapters are uploaded independently of execution status and the legacy report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSectionUpdate {
    pub id: Uuid,
    pub name: String,
    pub text: String,
    pub complete: bool,
    pub revision: u64,
    pub collected_at: i64,
}

pub const DIAGNOSTIC_SECTIONS_CAPABILITY: &str = "diagnostic:report-sections";
pub const DIAGNOSTIC_SECTION_LIMIT: usize = 64 * 1024;
pub const DIAGNOSTIC_SECTION_COUNT: usize = 32;

impl DiagnosticSectionUpdate {
    pub fn valid(&self) -> bool {
        !self.name.is_empty()
            && self.name.len() <= 64
            && self
                .name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            && !self.text.trim().is_empty()
            && self.text.len() <= DIAGNOSTIC_SECTION_LIMIT
            && (1..=i64::MAX as u64).contains(&self.revision)
            && self.collected_at > 0
    }
}
