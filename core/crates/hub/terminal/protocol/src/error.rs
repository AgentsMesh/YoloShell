use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, thiserror::Error)]
#[serde(rename_all = "snake_case")]
pub enum PublicError {
    #[error("unsupported terminal protocol schema")]
    UnsupportedSchema,
    #[error("invalid request identity")]
    InvalidRequest,
    #[error("working directory is unavailable; choose an existing directory on the selected Hub")]
    InvalidDirectory,
    #[error("this terminal requires explicit recreation; select recovery again to confirm a new shell")]
    RecreateRequired,
    #[error("terminal size exceeds protocol bounds")]
    InvalidSize,
    #[error("invalid terminal metadata")]
    InvalidMetadata,
    #[error("invalid secret capability")]
    InvalidCapability,
    #[error("invalid writer lease")]
    InvalidLease,
    #[error("invalid terminal payload")]
    InvalidPayload,
    #[error("invalid output cursor")]
    InvalidCursor,
    #[error("resource belongs to another origin")]
    OriginMismatch,
    #[error("operation targets another resource or runtime")]
    TargetMismatch,
    #[error("output is not contiguous; resynchronization is required")]
    OutputGap,
    #[error("resource was not found")]
    NotFound,
    #[error("operation is not authorized")]
    Unauthorized,
    #[error("resource is unreachable")]
    Unreachable,
    #[error("operation conflicts with current state")]
    Conflict,
    #[error("resource limit is exhausted")]
    LimitExceeded,
    #[error("operation is unsupported")]
    Unsupported,
    #[error("operation outcome is unknown")]
    OutcomeUnknown,
    #[error("origin could not complete the operation")]
    Internal,
}
