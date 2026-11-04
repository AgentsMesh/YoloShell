#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProtocolError {
    #[error("unsupported agent protocol schema")]
    UnsupportedSchema,
    #[error("invalid provider metadata")]
    InvalidProvider,
    #[error("invalid observed agent working directory")]
    InvalidWorkingDirectory,
    #[error("invalid observation cursor")]
    InvalidCursor,
    #[error("agent and terminal must belong to the same origin")]
    OriginMismatch,
    #[error("control targets another agent execution")]
    TargetMismatch,
    #[error("observation revision changed")]
    RevisionMismatch,
    #[error("observation is stale")]
    StaleObservation,
    #[error("origin is unreachable")]
    Unreachable,
    #[error("inconsistent agent state")]
    InvalidState,
    #[error("invalid interaction")]
    InvalidInteraction,
    #[error("invalid control request")]
    InvalidRequest,
    #[error("agent does not support this semantic control")]
    UnsupportedControl,
    #[error("semantic control is currently unavailable")]
    UnavailableControl,
    #[error("interaction is no longer current")]
    UnknownInteraction,
    #[error("response is not allowed for this interaction")]
    InvalidResponse,
    #[error("invalid command outcome transition")]
    InvalidTransition,
}
