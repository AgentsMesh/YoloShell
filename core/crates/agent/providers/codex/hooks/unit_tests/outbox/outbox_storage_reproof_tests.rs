use super::{envelope, identity};
use crate::codex_hooks::{CodexOutbox, CodexOutboxState};
use tempfile::TempDir;
use uuid::Uuid;

#[derive(Clone, Copy)]
enum InitialOutbox {
    Missing,
    Existing,
}

struct Fixture {
    _root: TempDir,
    outbox: CodexOutbox,
    owner: super::CodexHookIdentity,
}

impl Fixture {
    fn new(initial: InitialOutbox) -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .join(crate::codex_hooks::constants::OUTBOX_FILE_NAME);
        let outbox = CodexOutbox::new(&path).unwrap();
        if matches!(initial, InitialOutbox::Existing) {
            crate::paths::atomic_write_json(&path, &CodexOutboxState::default()).unwrap();
        }
        Self {
            _root: root,
            outbox,
            owner: identity(),
        }
    }

    fn arm_initial_sync_failure(&self) {
        crate::codex_hooks::set_outbox_commit_hook(self.outbox.path().to_path_buf(), || {
            crate::paths::test_support::fail_next_directory_sync();
        });
    }

    fn enqueue(&self) -> anyhow::Result<crate::agent::hooks::HookEnvelope> {
        self.outbox.enqueue(envelope(&self.owner, Uuid::new_v4()))
    }
}

#[test]
fn missing_target_sync_failure_is_upgraded_after_exact_reproof() {
    assert_sync_failure_recovered(InitialOutbox::Missing);
}

#[test]
fn existing_target_sync_failure_is_upgraded_after_exact_reproof() {
    assert_sync_failure_recovered(InitialOutbox::Existing);
}

#[test]
fn missing_namespace_after_failed_create_is_commit_indeterminate() {
    let fixture = Fixture::new(InitialOutbox::Missing);
    fixture.arm_initial_sync_failure();
    let path = fixture.outbox.path().to_path_buf();
    crate::codex_hooks::set_outbox_reproof_hook(path.clone(), move || {
        std::fs::remove_file(path).unwrap();
    });

    assert_commit_indeterminate(fixture.enqueue());
    assert!(!fixture.outbox.path().exists());
}

#[test]
fn old_namespace_after_failed_exchange_is_commit_indeterminate() {
    let fixture = Fixture::new(InitialOutbox::Existing);
    let old = CodexOutboxState::default();
    fixture.arm_initial_sync_failure();
    let path = fixture.outbox.path().to_path_buf();
    crate::codex_hooks::set_outbox_reproof_hook(path.clone(), move || {
        crate::paths::atomic_write_json(&path, &old).unwrap();
    });

    assert_commit_indeterminate(fixture.enqueue());
    assert_eq!(fixture.outbox.load().unwrap(), CodexOutboxState::default());
}

#[test]
fn retained_target_with_persistent_barrier_failure_is_commit_indeterminate() {
    let fixture = Fixture::new(InitialOutbox::Missing);
    fixture.arm_initial_sync_failure();
    crate::codex_hooks::set_outbox_reproof_hook(
        fixture.outbox.path().to_path_buf(),
        crate::paths::test_support::fail_next_directory_sync,
    );

    assert_commit_indeterminate(fixture.enqueue());
    assert_eq!(fixture.outbox.load().unwrap().pending.len(), 1);
}

fn assert_sync_failure_recovered(initial: InitialOutbox) {
    let fixture = Fixture::new(initial);
    fixture.arm_initial_sync_failure();

    let assigned = fixture.enqueue().unwrap();

    let state = fixture.outbox.load().unwrap();
    assert_eq!(state.pending.front(), Some(&assigned));
}

fn assert_commit_indeterminate(result: anyhow::Result<crate::agent::hooks::HookEnvelope>) {
    let error = result.unwrap_err();
    assert!(crate::codex_hooks::is_outbox_commit_indeterminate(&error));
}
