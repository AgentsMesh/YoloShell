use super::support::error_text;
use super::test_constants::GIT_QUIET_ARGUMENT;
use crate::snapshot::GateSnapshot;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use yolo_release_filesystem::constants::USER_WRITE_PERMISSION;
use yolo_release_workspace_state::WorkspaceState;
use yolo_release_workspace_state::constants::{GIT_EXECUTABLE, GIT_INIT_VERB};

const TRACKED_FILE: &str = "tracked.txt";
const TRACKED_BYTES: &[u8] = b"sealed source\n";
const MUTATED_BYTES: &[u8] = b"mutated source\n";

#[test]
fn source_rename_and_replacement_is_rejected() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let snapshot = GateSnapshot::create(&state).unwrap();
    let source = source_path(&snapshot);
    let moved = source.with_extension("moved");
    fs::rename(&source, &moved).unwrap();
    fs::create_dir(&source).unwrap();
    assert!(error_text(snapshot.verify_integrity()).contains("child directory identity"));
    fs::remove_dir(&source).unwrap();
    fs::rename(&moved, &source).unwrap();
}

#[test]
fn root_rename_and_replacement_is_rejected() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let snapshot = GateSnapshot::create(&state).unwrap();
    let root = source_path(&snapshot).parent().unwrap().to_owned();
    let moved = root.with_extension("moved");
    fs::rename(&root, &moved).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(error_text(snapshot.verify_integrity()).contains("path identity changed"));
    fs::remove_dir(&root).unwrap();
    fs::rename(&moved, &root).unwrap();
}

#[test]
fn sibling_rename_and_replacement_is_rejected() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let snapshot = GateSnapshot::create(&state).unwrap();
    let sibling = PathBuf::from(snapshot.execution().unwrap().output_user_root);
    let moved = sibling.with_extension("moved");
    fs::rename(&sibling, &moved).unwrap();
    fs::create_dir(&sibling).unwrap();
    assert!(error_text(snapshot.verify_integrity()).contains("child directory identity"));
    fs::remove_dir(&sibling).unwrap();
    fs::rename(&moved, &sibling).unwrap();
}

#[test]
fn chmod_mutate_and_restore_is_detected_by_ctime_manifest() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let snapshot = GateSnapshot::create(&state).unwrap();
    let tracked = source_path(&snapshot).join(TRACKED_FILE);
    let original_mode = fs::metadata(&tracked).unwrap().permissions().mode();
    fs::set_permissions(
        &tracked,
        fs::Permissions::from_mode(original_mode | USER_WRITE_PERMISSION),
    )
    .unwrap();
    fs::write(&tracked, MUTATED_BYTES).unwrap();
    fs::write(&tracked, TRACKED_BYTES).unwrap();
    fs::set_permissions(&tracked, fs::Permissions::from_mode(original_mode)).unwrap();
    assert!(error_text(snapshot.verify_integrity()).contains("source manifest changed"));
}

fn source_path(snapshot: &GateSnapshot) -> PathBuf {
    PathBuf::from(snapshot.execution().unwrap().workspace_snapshot)
}

fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &[GIT_INIT_VERB, GIT_QUIET_ARGUMENT]);
    git(root.path(), &["config", "user.name", "Yolo Test"]);
    git(
        root.path(),
        &["config", "user.email", "yolo@example.invalid"],
    );
    fs::write(root.path().join(TRACKED_FILE), TRACKED_BYTES).unwrap();
    git(root.path(), &["add", TRACKED_FILE]);
    git(
        root.path(),
        &["commit", GIT_QUIET_ARGUMENT, "-m", "fixture"],
    );
    root
}

fn git(workspace: &Path, arguments: &[&str]) {
    let status = Command::new(GIT_EXECUTABLE)
        .args(arguments)
        .current_dir(workspace)
        .status()
        .unwrap();
    assert!(status.success());
}
