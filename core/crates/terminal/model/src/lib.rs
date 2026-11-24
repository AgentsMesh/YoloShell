use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[cfg(test)]
extern crate self as yolo_terminal_model;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/lib.rs"]
mod public_unit_tests;

#[path = "constants.rs"]
mod constants;
#[path = "creation.rs"]
mod creation;
mod creation_query;
#[path = "fault_reap.rs"]
mod fault_reap;
#[path = "lease.rs"]
mod lease;
#[path = "product_lifecycle.rs"]
mod product_lifecycle;
#[path = "runtime.rs"]
mod runtime;
#[path = "terminal.rs"]
mod terminal;
#[path = "terminal_manifest_digest.rs"]
mod terminal_manifest_digest;
#[path = "terminal_runtime_status.rs"]
mod terminal_runtime_status;
#[path = "termination.rs"]
mod termination;

pub use creation::{
    CleanupTerminationOutcome, CreateTerminal, CreateTerminalResult, CreationCleanupArchiveRequest,
    CreationCleanupDiscardRequest, CreationCleanupTerminateRequest, FinalizeCreationRequest,
    TerminalCallerContext,
};
pub use creation_query::{CreationQueryOutcome, CreationQueryResult};
pub use fault_reap::{FaultReapProgress, FaultReapProgressKind, FaultReapStatus};
pub use lease::{WriterCapability, WriterLease, WriterLeaseGrant};
pub use product_lifecycle::{
    TerminalDiscardRequest, TerminalLifecycleSelection, TerminalTerminateRequest,
};
pub use runtime::{
    BootstrapDelivery, RuntimeRecord, RuntimeUpdateError, RuntimeView, WorkerHello,
    WorkerLaunchSpec,
};
pub use terminal::{Capabilities, OutputChunk, OutputGapPayload, TerminalSnapshot};
pub use terminal_manifest_digest::{TerminalManifestDigest, TerminalManifestDigestError};
pub use termination::{TerminationTombstone, TerminationTombstoneError};
pub use yolo_terminal_metadata_protocol::{
    NotificationKind, TerminalNotification, TerminalPresentation,
};

pub const PROTOCOL_VERSION: u16 = yolo_wire::PROTOCOL_VERSION;
pub use yolo_terminal_metadata_protocol::MAX_TERMINAL_TEXT_BYTES;
pub const DEFAULT_COLS: u16 = 120;
pub const DEFAULT_ROWS: u16 = 32;
pub const MIN_TERMINAL_COLUMNS: u16 = 2;
pub const MIN_TERMINAL_ROWS: u16 = 2;
pub const TERMINATION_TOMBSTONE_SCHEMA_VERSION: u16 = 2;

pub const fn normalize_terminal_columns(cols: u16) -> u16 {
    if cols < MIN_TERMINAL_COLUMNS {
        MIN_TERMINAL_COLUMNS
    } else {
        cols
    }
}

pub const fn normalize_terminal_rows(rows: u16) -> u16 {
    if rows < MIN_TERMINAL_ROWS {
        MIN_TERMINAL_ROWS
    } else {
        rows
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryState {
    #[default]
    Created,
    Reattached,
    Resumed,
    Recreated,
    Restoring,
    Uncertain,
    NeedsRecreate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalRuntimeStatus {
    Starting,
    Running,
    Finalizing,
    OutputBlocked,
    Exited,
    /// Both recorded runtime identities are gone without a trusted exit record.
    Lost,
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalManifest {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub cols: u16,
    pub rows: u16,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    pub archived: bool,
    pub recovery_state: RecoveryState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExitRecord {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub reason: ExitReason,
    pub observed_at_ms: i64,
    pub last_output_sequence: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompletedAttachmentView {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub runtime_epoch: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExitReason {
    Natural,
    UserTerminate,
    WorkerCrash,
    HostReboot,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalView {
    pub manifest: TerminalManifest,
    pub runtime: Option<RuntimeView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<yolo_agent_contract::AgentBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_attachment: Option<CompletedAttachmentView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostStatus {
    pub protocol_version: u16,
    pub boot_id: String,
    pub host_pid: u32,
    pub controller_generation: u64,
    pub home: PathBuf,
    pub capabilities: Vec<String>,
}

pub fn now_ms() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
