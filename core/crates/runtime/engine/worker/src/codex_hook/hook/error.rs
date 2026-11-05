use super::constants::{
    CODEX_ACK_IDENTITY, CODEX_ACK_INVALID, CODEX_ACK_PROCESS, CODEX_ACK_PROTOCOL,
    CODEX_ACK_REVOKED, CODEX_ACK_SEQUENCE, CODEX_AUTHORITY_LOST_ERROR,
    CODEX_AUTHORITY_SUPERSEDED_ERROR, CODEX_PROCESS_ERROR, CODEX_REDUCER_ERROR,
    CODEX_STORAGE_ERROR,
};
use crate::agent::hooks::HookViolation;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(super) enum CodexHookRejection {
    Identity(&'static str),
    Process(anyhow::Error),
    Outbox(&'static str),
    Storage(anyhow::Error),
    CommitIndeterminate(anyhow::Error),
    AuthorityLost(anyhow::Error),
    Superseded,
    Violation(HookViolation),
    Internal,
}

impl CodexHookRejection {
    pub(super) fn code(&self) -> &'static str {
        match self {
            Self::Identity(_) => CODEX_ACK_IDENTITY,
            Self::Process(_) => CODEX_ACK_PROCESS,
            Self::Outbox(_) | Self::Storage(_) | Self::CommitIndeterminate(_) => CODEX_ACK_PROTOCOL,
            Self::AuthorityLost(_) => CODEX_ACK_REVOKED,
            Self::Superseded => CODEX_ACK_REVOKED,
            Self::Violation(violation) => violation_code(*violation),
            Self::Internal => CODEX_ACK_PROTOCOL,
        }
    }
}

impl Display for CodexHookRejection {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Identity(detail) => formatter.write_str(detail),
            Self::Process(error) => write!(formatter, "{CODEX_PROCESS_ERROR}: {error:#}"),
            Self::Outbox(detail) => formatter.write_str(detail),
            Self::Storage(error) => write!(formatter, "{CODEX_STORAGE_ERROR}: {error:#}"),
            Self::CommitIndeterminate(error) => write!(
                formatter,
                "{}: {error:#}",
                super::constants::CODEX_COMMIT_INDETERMINATE_ERROR
            ),
            Self::AuthorityLost(error) => {
                write!(formatter, "{CODEX_AUTHORITY_LOST_ERROR}: {error:#}")
            }
            Self::Superseded => formatter.write_str(CODEX_AUTHORITY_SUPERSEDED_ERROR),
            Self::Violation(error) => write!(formatter, "{CODEX_REDUCER_ERROR}: {error}"),
            Self::Internal => formatter.write_str(CODEX_REDUCER_ERROR),
        }
    }
}

impl CodexHookRejection {
    pub(super) fn is_commit_indeterminate(&self) -> bool {
        matches!(self, Self::CommitIndeterminate(_))
    }
}

impl std::error::Error for CodexHookRejection {}

fn violation_code(violation: HookViolation) -> &'static str {
    match violation {
        HookViolation::InvalidIdentity
        | HookViolation::ProviderMismatch
        | HookViolation::VersionMismatch
        | HookViolation::SessionMismatch
        | HookViolation::GenerationMismatch
        | HookViolation::ProcessMismatch => CODEX_ACK_IDENTITY,
        HookViolation::SequenceGap | HookViolation::DuplicateConflict => CODEX_ACK_SEQUENCE,
        HookViolation::AuthorityExhausted
        | HookViolation::Revoked
        | HookViolation::ProcessAlreadyExited => CODEX_ACK_REVOKED,
        HookViolation::InvalidEnvelope
        | HookViolation::UnsupportedEvent
        | HookViolation::InvalidTopology
        | HookViolation::SessionNotStarted => CODEX_ACK_INVALID,
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/hook/error_inline_tests.rs"]
mod tests;
