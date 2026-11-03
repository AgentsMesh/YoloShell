use crate::ledger::{Operation, Phase, WORKTREE_LEDGER_FILE, WorktreeLedger};
use crate::{CreateWorktree, GitWorkspace, WorkspaceService};
use std::path::{Path, PathBuf};
use std::process::Command;
use yolo_runtime_protocol::{WorktreeCreateRequest, WorktreeRemoveRequest};

const GIT: &str = "/usr/bin/git";

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

fn create_request(root: &Path, id: &str) -> CreateWorktree {
    CreateWorktree {
        repository_root: root.to_path_buf(),
        worktree_id: id.into(),
        branch: None,
        base: None,
        trust_repository: true,
    }
}

fn phases(home: &Path, token: &str) -> Vec<String> {
    let text = std::fs::read_to_string(home.join(WORKTREE_LEDGER_FILE)).unwrap();
    text.lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| record["token"] == token)
        .filter_map(|record| record["phase"].as_str().map(str::to_owned))
        .collect()
}

#[test]
fn service_records_create_and_remove_outcomes() {
    let Some(workspace) = executor() else { return };
    let repo = repository();
    let root = repo.path().canonicalize().unwrap();
    let home = tempfile::tempdir().unwrap();
    let service = WorkspaceService::for_test(workspace, home.path());
    let token = "create:11111111-2222-3333-4444-555555555555";
    let created = service
        .create_worktree(WorktreeCreateRequest {
            repository_root: root.display().to_string(),
            worktree_id: "11111111-2222-3333-4444-555555555555".into(),
            branch: None,
            base: None,
            trust_repository: true,
        })
        .unwrap();
    assert_eq!(phases(home.path(), token), ["reserved", "linked"]);
    service
        .remove_worktree(WorktreeRemoveRequest {
            repository_root: root.display().to_string(),
            path: created.path.clone(),
        })
        .unwrap();
    let remove_token = format!("remove:{}", created.path);
    assert_eq!(
        phases(home.path(), &remove_token),
        ["reserved", "removed"]
    );
}

#[test]
fn restart_reconciles_reserved_create_as_linked_or_failed() {
    let Some(workspace) = executor() else { return };
    let repo = repository();
    let root = repo.path().canonicalize().unwrap();
    let home = tempfile::tempdir().unwrap();
    let created = workspace
        .create(&create_request(&root, "aaaaaaaa-2222-3333-4444-555555555555"))
        .unwrap();
    let linked_token = "create:aaaaaaaa-2222-3333-4444-555555555555";
    let failed_token = "create:bbbbbbbb-2222-3333-4444-555555555555";
    {
        let ledger = WorktreeLedger::open(home.path().join(WORKTREE_LEDGER_FILE)).unwrap();
        ledger
            .reserve(
                linked_token.into(),
                Operation::Create,
                &root,
                Path::new(&created.path),
                None,
            )
            .unwrap();
        ledger
            .reserve(
                failed_token.into(),
                Operation::Create,
                &root,
                &root.join("absent"),
                None,
            )
            .unwrap();
    }
    let _service = WorkspaceService::for_test(workspace, home.path());
    assert_eq!(
        phases(home.path(), linked_token).last().map(String::as_str),
        Some("linked")
    );
    assert_eq!(
        phases(home.path(), failed_token).last().map(String::as_str),
        Some("failed")
    );
}
