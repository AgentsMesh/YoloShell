use super::{ExitReason, now_ms};
use crate::TERMINATION_TOMBSTONE_SCHEMA_VERSION;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminationTombstone {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub reason: ExitReason,
    pub requested_at_ms: i64,
    pub force: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum TerminationTombstoneError {
    #[error("termination tombstone schema mismatch")]
    Schema,
    #[error("termination tombstone terminal mismatch")]
    Terminal,
    #[error("termination tombstone runtime mismatch")]
    Runtime,
    #[error("termination tombstone reason is invalid")]
    Reason,
}

impl TerminationTombstone {
    pub fn user_request(terminal_id: Uuid, runtime_epoch: Uuid, force: bool) -> Self {
        Self {
            schema_version: TERMINATION_TOMBSTONE_SCHEMA_VERSION,
            terminal_id,
            runtime_epoch,
            reason: ExitReason::UserTerminate,
            requested_at_ms: now_ms(),
            force,
        }
    }

    pub fn validate_for(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Result<(), TerminationTombstoneError> {
        if self.schema_version != TERMINATION_TOMBSTONE_SCHEMA_VERSION {
            return Err(TerminationTombstoneError::Schema);
        }
        if self.terminal_id != terminal_id {
            return Err(TerminationTombstoneError::Terminal);
        }
        if self.runtime_epoch != runtime_epoch {
            return Err(TerminationTombstoneError::Runtime);
        }
        if self.reason != ExitReason::UserTerminate {
            return Err(TerminationTombstoneError::Reason);
        }
        Ok(())
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/termination_inline_tests.rs"]
mod tests;
