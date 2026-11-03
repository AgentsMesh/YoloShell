use serde::{Deserialize, Serialize};
use uuid::Uuid;

use yolo_terminal_model::{TerminalLifecycleSelection, TerminalView};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSessionsRequest {
    #[serde(default)]
    pub include_archived: bool,
    #[serde(default)]
    pub require_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListSessionsResult {
    pub sessions: Vec<TerminalView>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResumeRequest {
    #[serde(flatten)]
    pub selection: TerminalLifecycleSelection,
    #[serde(default)]
    pub recreate: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReconcileResult {
    pub reattached: usize,
    pub exited: usize,
    pub unreachable: usize,
    pub needs_recreate: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HelloRequest {
    pub client_name: Option<String>,
    pub protocol_version: u16,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelloResult {
    pub server_name: String,
    pub protocol_version: u16,
    pub capabilities: Vec<String>,
}

impl ReconcileResult {
    pub fn merge(&mut self, other: Self) {
        self.reattached += other.reattached;
        self.exited += other.exited;
        self.unreachable += other.unreachable;
        self.needs_recreate += other.needs_recreate;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceInspectRequest {
    pub root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum GitProbe {
    Available { version: String },
    Unavailable { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceInspectResult {
    pub canonical_root: String,
    pub git: GitProbe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeCreateRequest {
    pub repository_root: String,
    pub worktree_id: String,
    pub branch: Option<String>,
    pub base: Option<String>,
    pub trust_repository: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeRemoveRequest {
    pub repository_root: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeListRequest {
    pub repository_root: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeRecord {
    pub path: String,
    pub branch: Option<String>,
    pub head: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeListResult {
    pub worktrees: Vec<WorktreeRecord>,
}

/// Read the retained output of a terminal whose runtime can no longer append.
/// The caller must name the exact archived runtime epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedHistoryRequest {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub max_bytes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedHistoryResult {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub data_base64: String,
    pub from_sequence: u64,
    pub next_sequence: u64,
    pub truncated: bool,
}
