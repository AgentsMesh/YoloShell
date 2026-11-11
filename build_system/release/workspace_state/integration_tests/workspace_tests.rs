use super::support::error_text;
use crate::WorkspaceState;
use crate::constants::GIT_QUIET_ARGUMENT;
use std::fs;
use yolo_release_workspace_state_test_support::{
    DIRTY_TRACKED_BYTES, TRACKED_FILE, git, repository,
};

#[test]
fn clean_git_root_produces_exact_workspace_state() {
    let root = repository();
    let state = WorkspaceState::validate(root.path()).unwrap();
    assert_eq!(state.root(), fs::canonicalize(root.path()).unwrap());
    assert_eq!(state.root_handle().display_path(), state.root());
    assert_eq!(state.revision().len(), 40);
    state.ensure_unchanged().unwrap();
}

#[test]
fn subdirectory_and_dirty_workspace_are_rejected() {
    let root = repository();
    let subdirectory = root.path().join("subdirectory");
    fs::create_dir(&subdirectory).unwrap();
    let subdirectory_error = error_text(WorkspaceState::validate(&subdirectory));
    assert!(
        subdirectory_error.contains("must be the Git root"),
        "{subdirectory_error}"
    );
    fs::write(root.path().join(TRACKED_FILE), DIRTY_TRACKED_BYTES).unwrap();
    let dirty_error = error_text(WorkspaceState::validate(root.path()));
    assert!(dirty_error.contains("not clean"), "{dirty_error}");
}

#[test]
fn revision_change_is_detected_after_state_is_pinned() {
    let root = repository();
    let state = WorkspaceState::validate(root.path()).unwrap();
    fs::write(root.path().join(TRACKED_FILE), b"second").unwrap();
    git(root.path(), &["add", TRACKED_FILE]);
    git(root.path(), &["commit", GIT_QUIET_ARGUMENT, "-m", "second"]);
    assert!(error_text(state.ensure_unchanged()).contains("workspace changed"));
}
