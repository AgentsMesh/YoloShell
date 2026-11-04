use crate::{CreateWorktree, GitWorkspace, WorktreeError};
use std::path::{Path, PathBuf};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::time::Duration;

const GIT: &str = "/usr/bin/git";
const SCRIPT_MODE: u32 = 0o755;
const TIMEOUT_MS: u64 = 100;

fn executor() -> Option<GitWorkspace> {
    Path::new(GIT)
        .is_file()
        .then(|| GitWorkspace::configured(PathBuf::from(GIT)))
}

fn repository() -> tempfile::TempDir {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    git(&path, &["init", "-q"]);
    git(&path, &["config", "user.email", "test@example.com"]);
    git(&path, &["config", "user.name", "Test"]);
    std::fs::write(path.join("README"), "test\n").unwrap();
    git(&path, &["add", "README"]);
    git(&path, &["commit", "-q", "-m", "initial"]);
    root
}

fn git(cwd: &Path, args: &[&str]) {
    let status = Command::new(GIT)
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("HOME", cwd)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

#[test]
fn disabled_executor_reports_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let error = GitWorkspace::disabled("not configured")
        .probe(root.path())
        .unwrap_err();
    assert!(matches!(error, WorktreeError::Unavailable(_)));
}

#[test]
fn create_list_and_remove_worktree() {
    let Some(workspace) = executor() else { return };
    let root = repository();
    let path = root.path().canonicalize().unwrap();
    assert!(workspace.probe(&path).unwrap().contains("git version"));
    let record = workspace
        .create(&CreateWorktree {
            repository_root: path.clone(),
            worktree_id: "11111111-2222-3333-4444-555555555555".into(),
            branch: None,
            base: None,
            trust_repository: true,
        })
        .unwrap();
    assert!(Path::new(&record.path).is_dir());
    assert_eq!(
        record.branch.as_deref(),
        Some("yolo/11111111-2222-3333-4444-555555555555")
    );
    assert_eq!(workspace.list(&path).unwrap().len(), 2);
    workspace.remove(&path, Path::new(&record.path)).unwrap();
    assert!(!Path::new(&record.path).exists());
    assert_eq!(workspace.list(&path).unwrap().len(), 1);
}

#[test]
fn create_requires_trust_and_valid_repository() {
    let Some(workspace) = executor() else { return };
    let root = tempfile::tempdir().unwrap();
    let path = root.path().canonicalize().unwrap();
    let mut request = CreateWorktree {
        repository_root: path.clone(),
        worktree_id: "alpha".into(),
        branch: Some("feature".into()),
        base: None,
        trust_repository: false,
    };
    assert!(matches!(
        workspace.create(&request),
        Err(WorktreeError::UntrustedRepository)
    ));
    request.trust_repository = true;
    assert!(matches!(
        workspace.create(&request),
        Err(WorktreeError::InvalidRepository(_))
    ));
    let repo = repository();
    request.repository_root = repo.path().canonicalize().unwrap();
    request.branch = Some("-bad".into());
    assert!(matches!(
        workspace.create(&request),
        Err(WorktreeError::InvalidBranch)
    ));
}

#[test]
fn existing_target_is_reported_without_touching_it() {
    let Some(workspace) = executor() else { return };
    let root = repository();
    let path = root.path().canonicalize().unwrap();
    let container = path.parent().unwrap().join(format!(
        ".{}.yolo-worktrees",
        path.file_name().unwrap().to_str().unwrap()
    ));
    let target = container.join("taken");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("keep"), "x").unwrap();
    let error = workspace
        .create(&CreateWorktree {
            repository_root: path,
            worktree_id: "taken".into(),
            branch: None,
            base: None,
            trust_repository: true,
        })
        .unwrap_err();
    assert!(matches!(error, WorktreeError::TargetExists(_)));
    assert_eq!(std::fs::read_to_string(target.join("keep")).unwrap(), "x");
}

#[test]
fn missing_target_removes_registration_and_can_be_recreated() {
    let Some(workspace) = executor() else { return };
    let root = repository();
    let path = root.path().canonicalize().unwrap();
    let request = CreateWorktree {
        repository_root: path.clone(),
        worktree_id: "22222222-3333-4444-5555-666666666666".into(),
        branch: None,
        base: None,
        trust_repository: true,
    };
    let record = workspace.create(&request).unwrap();
    std::fs::remove_dir_all(&record.path).unwrap();
    workspace.remove(&path, Path::new(&record.path)).unwrap();
    assert_eq!(workspace.list(&path).unwrap().len(), 1);
    let recreated = workspace.create(&request).unwrap();
    assert_eq!(recreated.path, record.path);
}

#[test]
fn timeout_terminates_the_git_process_group() {
    let root = tempfile::tempdir().unwrap();
    let script = root.path().join("hang.sh");
    std::fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
    let mut permissions = std::fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(SCRIPT_MODE);
    std::fs::set_permissions(&script, permissions).unwrap();
    let error = crate::command::run_with_deadline(
        &script,
        &[],
        root.path(),
        Duration::from_millis(TIMEOUT_MS),
    )
    .unwrap_err();
    assert!(matches!(error, WorktreeError::Timeout));
}
