use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

use crate::PublicError;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachSeed {
    #[default]
    Snapshot,
    History,
}

impl AttachSeed {
    pub fn is_snapshot(&self) -> bool {
        matches!(self, Self::Snapshot)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputCursor {
    pub target: RuntimeTarget,
    #[serde(deserialize_with = "crate::validation::deserialize_sequence")]
    pub after_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "AttachRequestFields", into = "AttachRequestFields")]
pub struct AttachRequest(AttachRequestFields);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AttachRequestFields {
    target: RuntimeTarget,
    cursor: Option<OutputCursor>,
    #[serde(default, skip_serializing_if = "AttachSeed::is_snapshot")]
    seed: AttachSeed,
}

impl AttachRequest {
    /// No cursor requests a snapshot; a cursor is scoped to exactly one runtime.
    pub fn new(target: RuntimeTarget, cursor: Option<OutputCursor>) -> Result<Self, PublicError> {
        Self::with_seed(target, cursor, AttachSeed::Snapshot)
    }

    /// A history request asks for one raw stream from the first retained
    /// sequence; the Hub falls back to a snapshot when the journal is compacted.
    pub fn history(
        target: RuntimeTarget,
        cursor: Option<OutputCursor>,
    ) -> Result<Self, PublicError> {
        Self::with_seed(target, cursor, AttachSeed::History)
    }

    pub fn with_seed(
        target: RuntimeTarget,
        cursor: Option<OutputCursor>,
        seed: AttachSeed,
    ) -> Result<Self, PublicError> {
        AttachRequestFields {
            target,
            cursor,
            seed,
        }
        .try_into()
    }

    pub fn target(self) -> RuntimeTarget {
        self.0.target
    }
    pub fn cursor(self) -> Option<OutputCursor> {
        self.0.cursor
    }
    pub fn seed(self) -> AttachSeed {
        self.0.seed
    }
}

impl TryFrom<AttachRequestFields> for AttachRequest {
    type Error = PublicError;

    fn try_from(fields: AttachRequestFields) -> Result<Self, Self::Error> {
        if let Some(cursor) = fields.cursor {
            if cursor.target != fields.target {
                return Err(PublicError::TargetMismatch);
            }
            if cursor.after_sequence == u64::MAX {
                return Err(PublicError::InvalidCursor);
            }
        }
        Ok(Self(fields))
    }
}

impl From<AttachRequest> for AttachRequestFields {
    fn from(value: AttachRequest) -> Self {
        value.0
    }
}
