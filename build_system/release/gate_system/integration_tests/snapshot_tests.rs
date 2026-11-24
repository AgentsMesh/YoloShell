use super::support::error_text;
use super::test_constants::{GIT_HARD_RESET_ARGUMENT, GIT_QUIET_ARGUMENT};
use crate::snapshot::GateSnapshot;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;
use yolo_release_filesystem::constants::WRITE_PERMISSION_MASK;
use yolo_release_workspace_state::WorkspaceState;
use yolo_release_workspace_state::constants::{GIT_EXECUTABLE, GIT_INIT_VERB};

const TRACKED_FILE: &str = "tracked.txt";
const COMMITTED_BYTES: &[u8] = b"committed\n";
const REPLACEMENT_BYTES: &[u8] = b"replacement\n";
const LIVE_BYTES: &[u8] = b"live mutation\n";

#[test]
fn private_export_ignores_live_info_attributes_and_local_attributes_config() {
    let repository = repository();
    let info_attributes = repository.path().join(".git/info/attributes");
    fs::write(&info_attributes, format!("{TRACKED_FILE} export-ignore\n")).unwrap();
    let external = repository.path().join(".git/local-attributes");
    fs::write(&external, format!("{TRACKED_FILE} export-ignore\n")).unwrap();
    git(
        repository.path(),
        &["config", "core.attributesFile", external.to_str().unwrap()],
    );
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let snapshot = GateSnapshot::create(&state).unwrap();
    assert_eq!(snapshot_file(&snapshot), COMMITTED_BYTES);
}

#[test]
fn private_export_ignores_live_replace_refs() {
    let repository = repository();
    let original = git(repository.path(), &["rev-parse", "HEAD"]);
    fs::write(repository.path().join(TRACKED_FILE), REPLACEMENT_BYTES).unwrap();
    git(repository.path(), &["add", TRACKED_FILE]);
    git(
        repository.path(),
        &["commit", GIT_QUIET_ARGUMENT, "-m", "replacement"],
    );
    let replacement = git(repository.path(), &["rev-parse", "HEAD"]);
    git(
        repository.path(),
        &["reset", GIT_HARD_RESET_ARGUMENT, original.trim()],
    );
    git(
        repository.path(),
        &["replace", original.trim(), replacement.trim()],
    );
    let state = WorkspaceState::validate(repository.path()).unwrap();
    assert_eq!(state.revision(), original.trim());
    let snapshot = GateSnapshot::create(&state).unwrap();
    assert_eq!(snapshot_file(&snapshot), COMMITTED_BYTES);
}

#[test]
fn exact_commit_snapshot_is_read_only_despite_later_live_mutation() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    fs::write(repository.path().join(TRACKED_FILE), LIVE_BYTES).unwrap();
    let snapshot = GateSnapshot::create(&state).unwrap();
    let workspace = snapshot_workspace(&snapshot);
    assert_eq!(
        fs::read(workspace.join(TRACKED_FILE)).unwrap(),
        COMMITTED_BYTES
    );
    assert_eq!(
        fs::metadata(workspace.join(TRACKED_FILE))
            .unwrap()
            .permissions()
            .mode()
            & WRITE_PERMISSION_MASK,
        0
    );
    assert!(state.ensure_unchanged().is_err());
}

#[test]
fn tracked_symlink_cannot_escape_snapshot() {
    let repository = repository();
    symlink("/tmp", repository.path().join("escape")).unwrap();
    git(repository.path(), &["add", "escape"]);
    git(
        repository.path(),
        &["commit", GIT_QUIET_ARGUMENT, "-m", "symlink"],
    );
    let state = WorkspaceState::validate(repository.path()).unwrap();
    assert!(error_text(GateSnapshot::create(&state)).contains("contains a symlink"));
}

#[test]
fn read_only_source_is_removed_when_snapshot_drops() {
    let repository = repository();
    let state = WorkspaceState::validate(repository.path()).unwrap();
    let root = {
        let snapshot = GateSnapshot::create(&state).unwrap();
        snapshot_workspace(&snapshot).parent().unwrap().to_owned()
    };
    assert!(!root.exists());
}

fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    git(root.path(), &[GIT_INIT_VERB, GIT_QUIET_ARGUMENT]);
    git(root.path(), &["config", "user.name", "Yolo Test"]);
    git(
        root.path(),
        &["config", "user.email", "yolo@example.invalid"],
    );
    fs::write(root.path().join(TRACKED_FILE), COMMITTED_BYTES).unwrap();
    git(root.path(), &["add", TRACKED_FILE]);
    git(
        root.path(),
        &["commit", GIT_QUIET_ARGUMENT, "-m", "fixture"],
    );
    root
}

fn snapshot_workspace(snapshot: &GateSnapshot) -> PathBuf {
    PathBuf::from(snapshot.execution().unwrap().workspace_snapshot)
}

fn snapshot_file(snapshot: &GateSnapshot) -> Vec<u8> {
    fs::read(snapshot_workspace(snapshot).join(TRACKED_FILE)).unwrap()
}

fn git(workspace: &Path, arguments: &[&str]) -> String {
    let output = Command::new(GIT_EXECUTABLE)
        .args(arguments)
        .current_dir(workspace)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
