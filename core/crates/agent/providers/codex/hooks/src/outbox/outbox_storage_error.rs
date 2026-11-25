use super::constants::errors;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) enum CodexOutboxStorageError {
    CommitIndeterminate(anyhow::Error),
}

impl Display for CodexOutboxStorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CommitIndeterminate(_) => {
                formatter.write_str(errors::OUTBOX_COMMIT_INDETERMINATE)
            }
        }
    }
}

impl std::error::Error for CodexOutboxStorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::CommitIndeterminate(error) => Some(error.as_ref()),
        }
    }
}

pub(super) fn commit_indeterminate(
    commit_error: anyhow::Error,
    reproof_error: anyhow::Error,
) -> anyhow::Error {
    let cause = reproof_error.context(format!(
        "{}: {commit_error:#}",
        errors::OUTBOX_COMMIT_FAILED
    ));
    CodexOutboxStorageError::CommitIndeterminate(cause).into()
}

pub fn is_commit_indeterminate(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<CodexOutboxStorageError>(),
        Some(CodexOutboxStorageError::CommitIndeterminate(_))
    )
}
