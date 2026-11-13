//! Public terminal resources and origin-fenced operations, independent of local runtime models.
//! Validation here never substitutes for origin authorization or worker write arbitration.

mod attach;
mod attach_result;
mod capability;
mod constants;
mod creation;
mod creation_query;
mod error;
mod history;
mod input;
mod lease;
mod lease_observation;
mod lifecycle;
mod run;
mod size;
mod summary;
mod text;
mod terminal;
mod terminal_event;
mod terminal_event_wire;
mod validation;

pub use attach::{AttachRequest, AttachSeed, OutputCursor};
pub use attach_result::{AttachResult, AttachResultFields};
pub use capability::SecretCapability;
pub use constants::*;
pub use creation::{
    CleanupTerminationOutcome, CreateShellFields, CreateShellRequest, CreateShellResult,
};
pub use creation_query::{CreationQueryOutcome, CreationQueryRequest, CreationQueryResult};
pub use error::PublicError;
pub use history::{
    DEFAULT_TERMINAL_HISTORY_BYTES, MAX_TERMINAL_HISTORY_BYTES, ReadTerminalHistoryFields,
    ReadTerminalHistoryRequest, TerminalHistoryRead, TerminalHistoryReadFields,
};
pub use input::{InputFields, InputRequest, ResizeRequest};
pub use lease::{LeaseGrant, LeaseGrantFields, LeaseMode, LeaseRequest, WriterPrincipal};
pub use lease_observation::{LeaseObservation, LeaseObservationFields};
pub use lifecycle::{TerminalAction, TerminalCommand, TerminalSelection};
pub use run::{TerminalRunFields, TerminalRunRequest, TerminalRunResult};
pub use size::TerminalSize;
pub use summary::{
    RuntimeStatus, RuntimeSummary, TerminalRecoveryHint, TerminalSummary, TerminalSummaryFields,
};
pub use text::{
    ReadTerminalTextFields, ReadTerminalTextRequest, TerminalText, TerminalTextFields,
};
pub use terminal::{
    OutputChunk, OutputChunkFields, TerminalHistory, TerminalHistoryFields, TerminalSnapshot,
    TerminalSnapshotFields,
};
pub use terminal_event::TerminalEvent;
pub use yolo_terminal_metadata_protocol::{
    NotificationKind, TerminalNotification, TerminalPresentation,
};
