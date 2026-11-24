mod authority;
mod commands;
mod constants;
mod control;
mod control_loop;
mod error;
mod lanes;
mod output;
mod run;
mod startup;

pub use commands::{CommandChannel, ControlCommand};
pub use constants::{
    COMMAND_QUEUE_CAPACITY, OUTPUT_TIMEOUT, REQUEST_TIMEOUT, STATUS_QUEUE_CAPACITY,
};
pub use error::RelayError;
pub use run::{run, run_with_commands};
use std::path::PathBuf;
use yolo_hub_identity::{HubId, RuntimeTarget};
use yolo_hub_terminal_protocol::{TerminalSize, WriterPrincipal};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlIntent {
    Observe,
    Acquire,
    TakeControl,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub socket: PathBuf,
    pub entry: HubId,
    pub target: RuntimeTarget,
    pub principal: WriterPrincipal,
    pub intent: ControlIntent,
    pub size: Option<TerminalSize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Status {
    ControlAcquired,
    ReadOnly { holder_id: Option<String> },
    ControlLost { holder_id: Option<String> },
    SessionExited,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Detached,
    Exited {
        code: Option<i32>,
        signal: Option<i32>,
    },
}

#[cfg(test)]
#[path = "../unit_tests/lib.rs"]
mod unit_tests;
