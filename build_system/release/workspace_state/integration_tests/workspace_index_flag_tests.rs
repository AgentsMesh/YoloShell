use super::support::error_text;
use crate::WorkspaceState;
use crate::constants::WORKSPACE_INDEX_FLAGS_ERROR;
use std::fs;
use yolo_release_workspace_state_test_support::{
    GIT_ASSUME_UNCHANGED_ARGUMENT, GIT_SKIP_WORKTREE_ARGUMENT, GIT_UPDATE_INDEX_ARGUMENT,
    HIDDEN_INDEX_BYTES, TRACKED_FILE, git, repository,
};

#[test]
fn assume_unchanged_and_skip_worktree_cannot_hide_a_dirty_file() {
    for flag in [GIT_ASSUME_UNCHANGED_ARGUMENT, GIT_SKIP_WORKTREE_ARGUMENT] {
        let root = repository();
        git(
            root.path(),
            &[GIT_UPDATE_INDEX_ARGUMENT, flag, TRACKED_FILE],
        );
        fs::write(root.path().join(TRACKED_FILE), HIDDEN_INDEX_BYTES).unwrap();
        let error = error_text(WorkspaceState::validate(root.path()));
        assert!(error.contains(WORKSPACE_INDEX_FLAGS_ERROR), "{error}");
    }
}

#[test]
fn hidden_index_flag_added_after_validation_fails_final_check() {
    let root = repository();
    let state = WorkspaceState::validate(root.path()).unwrap();
    git(
        root.path(),
        &[
            GIT_UPDATE_INDEX_ARGUMENT,
            GIT_ASSUME_UNCHANGED_ARGUMENT,
            TRACKED_FILE,
        ],
    );
    let error = error_text(state.ensure_unchanged());
    assert!(error.contains(WORKSPACE_INDEX_FLAGS_ERROR), "{error}");
}
