use std::fs;
use std::path::Path;
use std::process::Command;
use yolo_release_workspace_state::constants::{GIT_EXECUTABLE, GIT_INIT_VERB, GIT_QUIET_ARGUMENT};

pub const TRACKED_FILE: &str = "tracked";
pub const TRACKED_BYTES: &[u8] = b"first";
pub const GIT_UPDATE_INDEX_ARGUMENT: &str = "update-index";
pub const HIDDEN_INDEX_BYTES: &[u8] = b"hidden index change\n";
pub const DIRTY_TRACKED_BYTES: &[u8] = b"deliberately dirty workspace contents\n";
pub const PARALLEL_WORKSPACE_CASES: usize = 8;

pub fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &[GIT_INIT_VERB, GIT_QUIET_ARGUMENT]);
    git(root.path(), &["config", "user.name", "Yolo Test"]);
    git(
        root.path(),
        &["config", "user.email", "yolo@example.invalid"],
    );
    fs::write(root.path().join(TRACKED_FILE), TRACKED_BYTES).unwrap();
    git(root.path(), &["add", TRACKED_FILE]);
    git(root.path(), &["commit", GIT_QUIET_ARGUMENT, "-m", "first"]);
    root
}

pub fn git(workspace: &Path, arguments: &[&str]) {
    let status = Command::new(GIT_EXECUTABLE)
        .args(arguments)
        .current_dir(workspace)
        .status()
        .unwrap();
    assert!(status.success());
}
#[path = "constants.rs"]
mod constants;

pub use constants::{
    GIT_ASSUME_UNCHANGED_ARGUMENT, GIT_SKIP_WORKTREE_ARGUMENT, GIT_WORKTREE_ADD_ARGUMENTS,
};
