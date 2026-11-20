use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_identity::AgentTarget;

use crate::{FIRST_OBSERVATION_SEQUENCE, ProtocolError};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CursorFields", into = "CursorFields")]
pub struct ObservationCursor(CursorFields);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CursorFields {
    target: AgentTarget,
    stream: Uuid,
    sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CursorPosition {
    Duplicate,
    Next,
    Gap,
    Older,
    ResetRequired,
}

impl ObservationCursor {
    /// A stream belongs to one agent execution; origin journal resets change stream ID.
    pub fn new(target: AgentTarget, stream: Uuid, sequence: u64) -> Result<Self, ProtocolError> {
        CursorFields {
            target,
            stream,
            sequence,
        }
        .try_into()
    }

    pub fn target(self) -> AgentTarget {
        self.0.target
    }

    pub fn stream(self) -> Uuid {
        self.0.stream
    }

    pub fn sequence(self) -> u64 {
        self.0.sequence
    }

    pub fn position_after(self, previous: Self) -> CursorPosition {
        if self.target() != previous.target() || self.stream() != previous.stream() {
            return CursorPosition::ResetRequired;
        }
        match self.sequence().cmp(&previous.sequence()) {
            std::cmp::Ordering::Equal => CursorPosition::Duplicate,
            std::cmp::Ordering::Less => CursorPosition::Older,
            std::cmp::Ordering::Greater => {
                if self.sequence() - previous.sequence() == FIRST_OBSERVATION_SEQUENCE {
                    CursorPosition::Next
                } else {
                    CursorPosition::Gap
                }
            }
        }
    }
}

impl TryFrom<CursorFields> for ObservationCursor {
    type Error = ProtocolError;

    fn try_from(fields: CursorFields) -> Result<Self, Self::Error> {
        if fields.stream.is_nil() || fields.sequence < FIRST_OBSERVATION_SEQUENCE {
            return Err(ProtocolError::InvalidCursor);
        }
        Ok(Self(fields))
    }
}

impl From<ObservationCursor> for CursorFields {
    fn from(cursor: ObservationCursor) -> Self {
        cursor.0
    }
}
