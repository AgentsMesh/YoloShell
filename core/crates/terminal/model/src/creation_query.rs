use crate::CreateTerminalResult;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;
use yolo_wire::field;

const REDACTED_QUERY_TOKEN: &str = "[REDACTED]";

/// A point-in-time creation transaction observation, never a current runtime proof.
#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreationQueryResult {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub creation_token: Uuid,
    pub outcome: CreationQueryOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CreationQueryOutcome {
    /// Absence now cannot rule out a still-in-flight Create. Never authorize replay.
    Missing {},
    Pending {},
    /// Returns the original creation result, not a claim that its shell still runs.
    Ready {
        creation: CreateTerminalResult,
    },
    Finalized {
        creation: CreateTerminalResult,
    },
    Survivor {
        creation: CreateTerminalResult,
    },
    Archived {
        #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
        terminal_id: Uuid,
    },
}

impl CreationQueryResult {
    pub fn matches_request(&self, creation_token: Uuid) -> bool {
        if creation_token.is_nil() || self.creation_token != creation_token {
            return false;
        }
        match self.outcome {
            CreationQueryOutcome::Ready { creation }
            | CreationQueryOutcome::Finalized { creation }
            | CreationQueryOutcome::Survivor { creation } => {
                !creation.terminal_id.is_nil() && !creation.creation_capability.is_nil()
            }
            CreationQueryOutcome::Archived { terminal_id } => !terminal_id.is_nil(),
            _ => true,
        }
    }
}

impl fmt::Debug for CreationQueryResult {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CreationQueryResult")
            .field(field::CREATION_TOKEN, &REDACTED_QUERY_TOKEN)
            .field("outcome", &self.outcome)
            .finish()
    }
}
