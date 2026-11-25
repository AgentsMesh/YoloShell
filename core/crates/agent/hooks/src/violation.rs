use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HookViolation {
    InvalidIdentity,
    InvalidEnvelope,
    ProviderMismatch,
    VersionMismatch,
    SessionMismatch,
    GenerationMismatch,
    ProcessMismatch,
    SequenceGap,
    DuplicateConflict,
    UnsupportedEvent,
    InvalidTopology,
    SessionNotStarted,
    ProcessAlreadyExited,
    AuthorityExhausted,
    Revoked,
}

impl Display for HookViolation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "lifecycle hook violation: {self:?}")
    }
}

impl Error for HookViolation {}

impl HookViolation {
    pub const fn is_durable_rejection(self) -> bool {
        !matches!(
            self,
            Self::AuthorityExhausted | Self::ProcessAlreadyExited | Self::Revoked
        )
    }
}
