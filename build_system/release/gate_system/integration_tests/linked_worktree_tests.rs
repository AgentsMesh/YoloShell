use super::support::error_text;
use super::test_constants::{LINKED_WORKTREE_NAME, TAR_EXTRACT_STDOUT_ARGUMENTS};
use crate::private_git::PrivateGit;
use crate::snapshot::GateSnapshot;
use std::fs;
use std::process::Command;
use yolo_release_contracts::SOURCE_ARCHIVE_NAME;
use yolo_release_workspace_state::WorkspaceState;
use yolo_release_workspace_state::constants::{
    GIT_METADATA_DIRECTORY_NAME, TAR_EXECUTABLE, WORKSPACE_INDEX_FLAGS_ERROR,
};
use yolo_release_workspace_state_test_support::{
    GIT_SKIP_WORKTREE_ARGUMENT, GIT_UPDATE_INDEX_ARGUMENT, GIT_WORKTREE_ADD_ARGUMENTS,
    HIDDEN_INDEX_BYTES, TRACKED_BYTES, TRACKED_FILE, git, repository,
};

#[test]
fn linked_worktree_gitfile_materializes_and_exports_private_snapshot() {
    let repository = repository();
    let worktree_root = tempfile::tempdir().unwrap();
    let linked = worktree_root.path().join(LINKED_WORKTREE_NAME);
    let linked_text = linked.to_str().unwrap();
    let arguments = GIT_WORKTREE_ADD_ARGUMENTS
        .iter()
        .copied()
        .chain(std::iter::once(linked_text))
        .collect::<Vec<_>>();
    git(repository.path(), &arguments);
    assert!(
        fs::metadata(linked.join(GIT_METADATA_DIRECTORY_NAME))
            .unwrap()
            .is_file()
    );

    let state = WorkspaceState::validate(&linked).unwrap();
    let snapshot = GateSnapshot::empty().unwrap();
    let private =
        PrivateGit::materialize(&state, &snapshot.root, &snapshot.git, &snapshot.temporary)
            .unwrap();
    private.ensure_complete().unwrap();
    private.export().unwrap();
    let output = Command::new(TAR_EXECUTABLE)
        .args(TAR_EXTRACT_STDOUT_ARGUMENTS)
        .arg(snapshot.root.display_path().join(SOURCE_ARCHIVE_NAME))
        .arg(TRACKED_FILE)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, TRACKED_BYTES);
}

#[test]
fn linked_worktree_skip_flag_cannot_bypass_clean_validation() {
    let repository = repository();
    let worktree_root = tempfile::tempdir().unwrap();
    let linked = worktree_root.path().join(LINKED_WORKTREE_NAME);
    let linked_text = linked.to_str().unwrap();
    let arguments = GIT_WORKTREE_ADD_ARGUMENTS
        .iter()
        .copied()
        .chain(std::iter::once(linked_text))
        .collect::<Vec<_>>();
    git(repository.path(), &arguments);
    git(
        &linked,
        &[
            GIT_UPDATE_INDEX_ARGUMENT,
            GIT_SKIP_WORKTREE_ARGUMENT,
            TRACKED_FILE,
        ],
    );
    fs::write(linked.join(TRACKED_FILE), HIDDEN_INDEX_BYTES).unwrap();
    let error = error_text(WorkspaceState::validate(&linked));
    assert!(error.contains(WORKSPACE_INDEX_FLAGS_ERROR), "{error}");
}
