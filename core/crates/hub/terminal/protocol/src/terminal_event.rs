use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

use crate::terminal_event_wire::TerminalEventWire;
use crate::{FIRST_OUTPUT_SEQUENCE, MAX_EXIT_CODE, OutputChunk, OutputCursor, PublicError};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "TerminalEventWire", into = "TerminalEventWire")]
pub enum TerminalEvent {
    Output {
        chunk: OutputChunk,
    },
    Heartbeat {
        cursor: OutputCursor,
    },
    Exited {
        cursor: OutputCursor,
        code: Option<i32>,
        signal: Option<i32>,
    },
    ResyncRequired {
        target: RuntimeTarget,
    },
    Unreachable {
        target: RuntimeTarget,
    },
}

impl TerminalEvent {
    pub fn advance(&self, cursor: OutputCursor) -> Result<OutputCursor, PublicError> {
        self.validate()?;
        if cursor.after_sequence == u64::MAX {
            return Err(PublicError::InvalidCursor);
        }
        if self.target() != cursor.target {
            return Err(PublicError::TargetMismatch);
        }
        match self {
            Self::Output { chunk } => {
                if cursor.after_sequence.checked_add(FIRST_OUTPUT_SEQUENCE)
                    != Some(chunk.fields().sequence)
                {
                    return Err(PublicError::OutputGap);
                }
                Ok(OutputCursor {
                    target: cursor.target,
                    after_sequence: chunk.fields().sequence,
                })
            }
            Self::Heartbeat { cursor: current }
            | Self::Exited {
                cursor: current, ..
            } => {
                if *current != cursor {
                    return Err(PublicError::OutputGap);
                }
                Ok(cursor)
            }
            Self::ResyncRequired { .. } => Err(PublicError::OutputGap),
            Self::Unreachable { .. } => Err(PublicError::Unreachable),
        }
    }

    pub fn validate(&self) -> Result<(), PublicError> {
        if let Self::Heartbeat { cursor } | Self::Exited { cursor, .. } = self {
            if cursor.after_sequence == u64::MAX {
                return Err(PublicError::InvalidCursor);
            }
        }
        if let Self::Exited { code, signal, .. } = self {
            if code.is_some_and(|code| !(0..=MAX_EXIT_CODE).contains(&code))
                || signal.is_some_and(|signal| signal <= 0)
                || (code.is_some() && signal.is_some())
            {
                return Err(PublicError::InvalidPayload);
            }
        }
        Ok(())
    }

    pub fn target(&self) -> RuntimeTarget {
        match self {
            Self::Output { chunk } => chunk.fields().target,
            Self::Heartbeat { cursor } | Self::Exited { cursor, .. } => cursor.target,
            Self::ResyncRequired { target } | Self::Unreachable { target } => *target,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Exited { .. } | Self::ResyncRequired { .. } | Self::Unreachable { .. }
        )
    }
}
