//! Client-facing Agent observations and conditional semantic controls.
//! Provider hooks remain authenticated adapter input, never public state mutations.
//! Origin authorization and terminal write arbitration are required separately.

mod capability;
mod constants;
mod control;
mod cursor;
mod directory;
mod error;
mod interaction;
mod observed_provider;
mod outcome;
mod prompt;
mod snapshot;
mod start;
mod state;
mod stop;
mod wait;

pub use capability::{AgentCapabilities, Capability, UnavailabilityReason};
pub use constants::*;
pub use control::{AgentAction, AgentControlFields, AgentControlRequest, InteractionResponse};
pub use cursor::{CursorPosition, ObservationCursor};
pub use directory::AgentWorkingDirectory;
pub use error::ProtocolError;
pub use interaction::{Interaction, InteractionChoice, InteractionKind, InteractionReference};
pub use observed_provider::ObservedProvider;
pub use outcome::{CommandOutcome, CommandReceipt, FailureCode};
pub use prompt::{
    AgentPromptFields, AgentPromptOutcome, AgentPromptReference, AgentPromptRequest,
    AgentPromptResult, AgentPromptWait, MAX_PROMPT_WAIT_MS,
};
pub use snapshot::{AgentSnapshot, AgentSnapshotFields};
pub use start::{AgentStartBinding, AgentStartFields, AgentStartRequest, AgentStartResult};
pub use state::{Activity, Freshness, Lifecycle, ObservationSource, Reachability, RecoveryStatus};
pub use stop::{AgentStopFields, AgentStopOutcome, AgentStopRequest, AgentStopResult};
pub use wait::{
    AgentLifecycleState, AgentWaitCondition, AgentWaitRequest, AgentWaitResult,
    MAX_AGENT_WAIT_DEPTH, MAX_AGENT_WAIT_FILTERS, MAX_AGENT_WAIT_MS, MAX_AGENT_WAIT_TARGETS,
};
pub use yolo_agent_input::{MAX_PROMPT_BYTES, Prompt};
