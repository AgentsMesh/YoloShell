use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::{RuntimeEpoch, TerminalAddress};

use crate::{PublicError, TerminalSummary, validation};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalSelection {
    pub address: TerminalAddress,
    /// None means the caller observed no runtime, never an unfenced wildcard.
    pub expected_runtime: Option<RuntimeEpoch>,
}

impl TerminalSelection {
    pub fn validate_against(self, current: &TerminalSummary) -> Result<(), PublicError> {
        let current = current.fields();
        if self.address != current.address
            || self.expected_runtime != current.runtime.map(|runtime| runtime.target.runtime_epoch)
        {
            return Err(PublicError::TargetMismatch);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum TerminalAction {
    Resume {},
    Recreate {},
    Terminate { force: bool },
    Archive {},
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalCommand {
    #[serde(
        serialize_with = "validation::serialize_id",
        deserialize_with = "validation::deserialize_id"
    )]
    pub request_id: Uuid,
    pub selection: TerminalSelection,
    pub action: TerminalAction,
}

impl TerminalCommand {
    pub fn validate(&self) -> Result<(), PublicError> {
        validation::request_id(self.request_id)
    }
}
