use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

use super::AgentKind;
use crate::{ZERO_PROCESS_GROUP, ZERO_PROCESS_START_TIME};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// The authenticated provider process is alive; its current activity is unconfirmed.
    Active,
    Running,
    WaitingForInput,
    WaitingForPermission,
    Exited,
    Unknown,
}

impl AgentState {
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Active | Self::Running | Self::WaitingForInput | Self::WaitingForPermission
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentRecoveryAvailability {
    Available,
    UnmanagedLaunch,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentBinding {
    pub schema_version: u32,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    pub kind: AgentKind,
    pub provider_session_id: String,
    pub executable: String,
    pub cwd: String,
    pub evidence_path: String,
    pub data_home: String,
    pub process_pid: Option<u32>,
    pub process_start_time: Option<u64>,
    pub state: AgentState,
    pub recovery_availability: AgentRecoveryAvailability,
    pub detected_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentEvidence {
    pub provider_session_id: String,
    pub cwd: PathBuf,
    pub path: PathBuf,
    pub data_home: PathBuf,
    pub state: AgentState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionResult {
    BindingChanged(AgentBinding),
    StatusChanged(AgentBinding),
    Unchanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedProcess {
    pub pid: u32,
    pub start_time: u64,
    pub parent_pid: u32,
    pub process_group: i32,
    pub foreground_group: i32,
    pub executable: String,
    pub argv: Vec<String>,
    pub environment: Option<BTreeMap<String, String>>,
    pub cwd: Option<PathBuf>,
    pub evidence_paths: Option<Vec<PathBuf>>,
}

impl ObservedProcess {
    pub fn is_foreground(&self) -> bool {
        self.foreground_group > ZERO_PROCESS_GROUP && self.process_group == self.foreground_group
    }

    pub fn identity_matches(&self, pid: u32, start_time: u64) -> bool {
        self.pid == pid && self.start_time == start_time && start_time != ZERO_PROCESS_START_TIME
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumePlan {
    pub executable: String,
    pub args: Vec<String>,
    pub environment: BTreeMap<String, String>,
}
