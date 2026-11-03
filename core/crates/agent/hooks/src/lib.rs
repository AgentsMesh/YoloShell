#[path = "constants.rs"]
pub mod constants;
#[path = "event.rs"]
pub mod event;
#[path = "identity.rs"]
pub mod identity;
#[path = "mapping.rs"]
pub mod mapping;
#[path = "state.rs"]
pub mod state;
#[path = "violation.rs"]
pub mod violation;

pub use constants::{
    AUTHORITY_REVISION_STEP, CURRENT_SCHEMA_VERSION, FIRST_SEQUENCE, INITIAL_AUTHORITY_REVISION,
    INITIAL_GENERATION, MAX_HOOK_ENVELOPE_BYTES, MAX_HOOK_IDENTIFIER_BYTES,
    MAX_HOOK_METADATA_PATH_BYTES, MAX_REPLAY_WINDOW, MAX_RETAINED_LIFECYCLE_ENTRIES,
    REDUCER_SNAPSHOT_SCHEMA_VERSION, ZERO_SEQUENCE, event_names, provider_names,
};
pub use event::{
    EventHistory, EventIds, EventKind, HookEnvelope, HookMetadata, NotificationKind,
    SessionEndReason, SessionStartSource,
};
pub use identity::{
    EventSequence, ExpectedIdentity, Generation, HookIdentity, ProcessIdentity, HookProviderKind,
    ProviderVersion,
};
pub use mapping::{ClaudeEventMapper, CodexEventMapper, EventMapper, ProviderEventMapper};
pub use state::{
    AttentionState, ChildLifecycle, InstrumentationState, ProcessState, StopCandidate,
    TerminationCandidate, ToolSet, TurnState,
};
pub use violation::HookViolation;
