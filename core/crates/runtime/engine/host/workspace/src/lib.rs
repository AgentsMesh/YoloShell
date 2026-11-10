//! Optional Host git executor for workspace worktrees. The executor is
//! configured by absolute path; when absent, every worktree operation is an
//! explicit `Unavailable` rather than a partial filesystem effect.

mod command;
mod constants;
mod error;
mod ledger;
mod ledger_operations;
mod operations;
mod parse;
mod service;
mod validate;

pub use error::WorktreeError;
pub use service::{WorkspaceFailure, WorkspaceService};

use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorktreeRecord {
    pub path: String,
    pub branch: Option<String>,
    pub head: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CreateWorktree {
    pub repository_root: PathBuf,
    pub worktree_id: String,
    pub branch: Option<String>,
    pub base: Option<String>,
    pub trust_repository: bool,
}

#[derive(Clone, Debug)]
pub struct GitWorkspace {
    executable: Option<PathBuf>,
    unavailable: String,
}

impl GitWorkspace {
    pub fn from_environment() -> Self {
        let Some(value) = std::env::var_os("YOLO_GIT_EXECUTABLE") else {
            return Self::disabled("git executor is not configured");
        };
        let path = PathBuf::from(value);
        if !path.is_absolute() || !path.is_file() {
            return Self::disabled("configured git executable is not an absolute file");
        }
        Self {
            executable: Some(path),
            unavailable: String::new(),
        }
    }

    /// Test/composition-only constructor for an already validated executable.
    pub fn configured(executable: std::path::PathBuf) -> Self {
        Self {
            executable: Some(executable),
            unavailable: String::new(),
        }
    }

    pub fn disabled(reason: &str) -> Self {
        Self {
            executable: None,
            unavailable: reason.into(),
        }
    }

    pub fn probe(&self, root: &Path) -> Result<String, WorktreeError> {
        let executable = self
            .executable
            .as_deref()
            .ok_or_else(|| WorktreeError::Unavailable(self.unavailable.clone()))?;
        let version = command::run(executable, &[constants::GIT_VERSION_ARGUMENT.into()], root)?;
        let repository = repository_root(executable, root)?;
        if repository == root {
            Ok(version.trim().to_owned())
        } else {
            Err(WorktreeError::InvalidRepository(root.display().to_string()))
        }
    }
}

use validate::repository_root;

pub(crate) fn cleanup_failed_target(target: &Path) {
    if target.is_dir() && std::fs::read_dir(target).is_ok_and(|mut d| d.next().is_none()) {
        let _ = std::fs::remove_dir(target);
    }
}

pub(crate) fn io(error: std::io::Error) -> WorktreeError {
    WorktreeError::Io(error.to_string())
}

#[cfg(test)]
#[path = "../unit_tests/worktree.rs"]
mod unit_tests;

#[cfg(test)]
#[path = "../unit_tests/ledger.rs"]
mod ledger_tests;
