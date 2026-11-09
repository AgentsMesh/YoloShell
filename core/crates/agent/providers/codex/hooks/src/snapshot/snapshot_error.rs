use super::constants::errors::{
    SNAPSHOT_DIRECTORY_CHANGED, SNAPSHOT_INDETERMINATE, SNAPSHOT_LOCK_CHANGED,
    SNAPSHOT_TARGET_CHANGED,
};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) enum CodexSnapshotFenceError {
    SnapshotChanged,
    LockPathChanged,
    InvocationDirectoryChanged,
    Indeterminate(anyhow::Error),
}

impl Display for CodexSnapshotFenceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::SnapshotChanged => SNAPSHOT_TARGET_CHANGED,
            Self::LockPathChanged => SNAPSHOT_LOCK_CHANGED,
            Self::InvocationDirectoryChanged => SNAPSHOT_DIRECTORY_CHANGED,
            Self::Indeterminate(_) => SNAPSHOT_INDETERMINATE,
        })
    }
}

impl std::error::Error for CodexSnapshotFenceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Indeterminate(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

pub(super) fn snapshot_changed() -> anyhow::Error {
    CodexSnapshotFenceError::SnapshotChanged.into()
}

pub(super) fn lock_changed() -> anyhow::Error {
    CodexSnapshotFenceError::LockPathChanged.into()
}

pub(super) fn directory_changed() -> anyhow::Error {
    CodexSnapshotFenceError::InvocationDirectoryChanged.into()
}

pub(super) fn indeterminate(error: anyhow::Error) -> anyhow::Error {
    CodexSnapshotFenceError::Indeterminate(error).into()
}

pub(super) fn is_fence(error: &anyhow::Error) -> bool {
    error.downcast_ref::<CodexSnapshotFenceError>().is_some()
}

pub(super) fn is_indeterminate(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::Indeterminate(_))
    )
}

pub fn is_authority_changed(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(
            CodexSnapshotFenceError::SnapshotChanged
                | CodexSnapshotFenceError::LockPathChanged
                | CodexSnapshotFenceError::InvocationDirectoryChanged
        )
    )
}
