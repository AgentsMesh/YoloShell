use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::AgentTarget;

use crate::{AgentControlRequest, ProtocolError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum CommandOutcome {
    Accepted {},
    Delivered {},
    Confirmed {},
    Failed { code: FailureCode },
    Unknown {},
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    PermissionDenied,
    PreconditionsChanged,
    Unsupported,
    AdapterRejected,
    Expired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReceiptFields", into = "ReceiptFields")]
pub struct CommandReceipt(ReceiptFields);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptFields {
    request_id: Uuid,
    target: AgentTarget,
    outcome: CommandOutcome,
}

impl CommandReceipt {
    /// A definitive rejection before dispatch, bound to an already validated request.
    pub fn rejected(request: &AgentControlRequest, code: FailureCode) -> Self {
        Self(ReceiptFields {
            request_id: request.fields().request_id,
            target: request.fields().target,
            outcome: CommandOutcome::Failed { code },
        })
    }

    pub fn accepted(request_id: Uuid, target: AgentTarget) -> Result<Self, ProtocolError> {
        ReceiptFields {
            request_id,
            target,
            outcome: CommandOutcome::Accepted {},
        }
        .try_into()
    }

    pub fn request_id(self) -> Uuid {
        self.0.request_id
    }

    pub fn target(self) -> AgentTarget {
        self.0.target
    }

    pub fn outcome(self) -> CommandOutcome {
        self.0.outcome
    }

    /// Unknown is not permission to retry; later origin evidence may resolve it.
    pub fn advance(&mut self, next: CommandOutcome) -> Result<(), ProtocolError> {
        if self.0.outcome == next {
            return Ok(());
        }
        let allowed = match self.0.outcome {
            CommandOutcome::Accepted {} => true,
            CommandOutcome::Delivered {} | CommandOutcome::Unknown {} => {
                next != CommandOutcome::Accepted {}
            }
            CommandOutcome::Confirmed {} | CommandOutcome::Failed { .. } => false,
        };
        if !allowed {
            return Err(ProtocolError::InvalidTransition);
        }
        self.0.outcome = next;
        Ok(())
    }
}

impl TryFrom<ReceiptFields> for CommandReceipt {
    type Error = ProtocolError;

    fn try_from(fields: ReceiptFields) -> Result<Self, Self::Error> {
        if fields.request_id.is_nil() {
            return Err(ProtocolError::InvalidRequest);
        }
        Ok(Self(fields))
    }
}

impl From<CommandReceipt> for ReceiptFields {
    fn from(receipt: CommandReceipt) -> Self {
        receipt.0
    }
}
