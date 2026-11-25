use super::{normalize_terminal_columns, normalize_terminal_rows};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

#[path = "runtime_capability_wire.rs"]
mod runtime_capability_wire;
#[path = "runtime_view.rs"]
mod runtime_view;
pub use runtime_view::RuntimeView;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapDelivery {
    NotRequired,
    Pending,
    Delivered,
    Survivor,
    Failed,
    Unknown,
}

impl BootstrapDelivery {
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::NotRequired | Self::Delivered)
    }

    pub const fn is_serviceable(self) -> bool {
        self.is_ready() || matches!(self, Self::Survivor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeUpdateError {
    #[error("runtime process identity changed")]
    ProcessIdentity,
    #[error("runtime launch identity changed")]
    LaunchIdentity,
    #[error("runtime controller generation regressed")]
    ControllerGenerationRegression,
    #[error("runtime terminal dimensions are not normalized")]
    NonCanonicalTerminalDimensions,
    #[error("runtime bootstrap delivery transition is invalid")]
    BootstrapDeliveryTransition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeRecord {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub worker_id: Uuid,
    pub worker_pid: u32,
    pub worker_start_time: u64,
    pub child_pid: u32,
    pub child_start_time: u64,
    pub boot_id: String,
    pub socket_path: PathBuf,
    pub auth_token: String,
    pub controller_generation: u64,
    pub started_at_ms: i64,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub cols: u16,
    pub rows: u16,
    pub bootstrap_delivery: BootstrapDelivery,
}

impl RuntimeRecord {
    /// Exact worker runtime identity used by callback ingresses. Callers hold
    /// the record lock, so this must stay a pure field comparison; it must not
    /// call back into process accessors that re-lock the same mutex.
    pub fn matches_worker_runtime(
        &self,
        runtime_epoch: Uuid,
        worker_pid: u32,
        worker_start_time: u64,
    ) -> bool {
        (self.runtime_epoch, self.worker_pid, self.worker_start_time)
            == (runtime_epoch, worker_pid, worker_start_time)
    }

    pub fn has_same_process_identity(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && self.terminal_id == other.terminal_id
            && self.runtime_epoch == other.runtime_epoch
            && self.worker_id == other.worker_id
            && self.worker_pid == other.worker_pid
            && self.worker_start_time == other.worker_start_time
            && self.child_pid == other.child_pid
            && self.child_start_time == other.child_start_time
            && self.boot_id == other.boot_id
            && self.socket_path == other.socket_path
            && self.auth_token == other.auth_token
    }

    pub fn validate_update_from(&self, current: &Self) -> Result<(), RuntimeUpdateError> {
        if !current.has_same_process_identity(self) {
            return Err(RuntimeUpdateError::ProcessIdentity);
        }
        if self.started_at_ms != current.started_at_ms
            || self.command != current.command
            || self.args != current.args
            || self.cwd != current.cwd
        {
            return Err(RuntimeUpdateError::LaunchIdentity);
        }
        if self.controller_generation < current.controller_generation {
            return Err(RuntimeUpdateError::ControllerGenerationRegression);
        }
        if self.cols != normalize_terminal_columns(self.cols)
            || self.rows != normalize_terminal_rows(self.rows)
        {
            return Err(RuntimeUpdateError::NonCanonicalTerminalDimensions);
        }
        if current.bootstrap_delivery != BootstrapDelivery::Pending
            && self.bootstrap_delivery != current.bootstrap_delivery
        {
            return Err(RuntimeUpdateError::BootstrapDeliveryTransition);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerHello {
    pub runtime: RuntimeRecord,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerLaunchSpec {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub worker_id: Uuid,
    pub socket_path: PathBuf,
    pub auth_token: String,
    pub controller_generation: u64,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    /// Provider runtime variables are single-use launch evidence, not shell metadata.
    pub environment: BTreeMap<String, String>,
    pub initial_input: Option<Vec<u8>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<crate::TerminalCallerContext>,
    pub cols: u16,
    pub rows: u16,
}
