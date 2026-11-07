use crate::constants::*;

/// Typed worktree failures. The Host maps these to wire error codes; the Hub
/// never sees a raw git message as authority for a partial success.
#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    #[error("git executor is not configured: {0}")]
    Unavailable(String),
    #[error("not a git repository root: {0}")]
    InvalidRepository(String),
    #[error("repository hooks were not trusted by the caller")]
    UntrustedRepository,
    #[error("invalid branch name")]
    InvalidBranch,
    #[error("invalid base revision")]
    InvalidBase,
    #[error("worktree target already exists: {0}")]
    TargetExists(String),
    #[error("path is not a registered worktree of this repository: {0}")]
    NotRegistered(String),
    #[error("git failed: {0}")]
    GitFailed(String),
    #[error("git did not finish before the deadline")]
    Timeout,
    #[error("git output exceeded the limit")]
    OutputTooLarge,
    #[error("git outcome is unknown: {0}")]
    Unknown(String),
    #[error("worktree io failed: {0}")]
    Io(String),
}

impl WorktreeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable(_) => WORKTREE_UNAVAILABLE,
            Self::InvalidRepository(_) => WORKTREE_INVALID_REPOSITORY,
            Self::UntrustedRepository => WORKTREE_UNTRUSTED_REPOSITORY,
            Self::InvalidBranch | Self::InvalidBase => WORKTREE_INVALID_BRANCH,
            Self::TargetExists(_) => WORKTREE_TARGET_EXISTS,
            Self::NotRegistered(_) => WORKTREE_NOT_REGISTERED,
            Self::GitFailed(_) => WORKTREE_GIT_FAILED,
            Self::Timeout => WORKTREE_TIMEOUT,
            Self::OutputTooLarge => WORKTREE_OUTPUT_TOO_LARGE,
            Self::Unknown(_) => WORKTREE_UNKNOWN,
            Self::Io(_) => WORKTREE_IO_FAILED,
        }
    }
}
