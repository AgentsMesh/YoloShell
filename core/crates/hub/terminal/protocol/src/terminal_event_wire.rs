use crate::{OutputChunk, OutputCursor, PublicError, TerminalEvent};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::RuntimeTarget;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum TerminalEventWire {
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

impl TryFrom<TerminalEventWire> for TerminalEvent {
    type Error = PublicError;
    fn try_from(wire: TerminalEventWire) -> Result<Self, Self::Error> {
        let event = match wire {
            TerminalEventWire::Output { chunk } => Self::Output { chunk },
            TerminalEventWire::Heartbeat { cursor } => Self::Heartbeat { cursor },
            TerminalEventWire::Exited {
                cursor,
                code,
                signal,
            } => Self::Exited {
                cursor,
                code,
                signal,
            },
            TerminalEventWire::ResyncRequired { target } => Self::ResyncRequired { target },
            TerminalEventWire::Unreachable { target } => Self::Unreachable { target },
        };
        event.validate()?;
        Ok(event)
    }
}

impl From<TerminalEvent> for TerminalEventWire {
    fn from(event: TerminalEvent) -> Self {
        match event {
            TerminalEvent::Output { chunk } => Self::Output { chunk },
            TerminalEvent::Heartbeat { cursor } => Self::Heartbeat { cursor },
            TerminalEvent::Exited {
                cursor,
                code,
                signal,
            } => Self::Exited {
                cursor,
                code,
                signal,
            },
            TerminalEvent::ResyncRequired { target } => Self::ResyncRequired { target },
            TerminalEvent::Unreachable { target } => Self::Unreachable { target },
        }
    }
}
