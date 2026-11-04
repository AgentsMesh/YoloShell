use crate::{constants::*, fixture::*};
use std::os::unix::fs::PermissionsExt;
use uuid::Uuid;
use yolo_agent_control_journal::{HEADER_FILE, JOURNAL_FILE};
use yolo_agent_control_runtime::*;
use yolo_paths::constants::PRIVATE_FILE_MODE;

#[test]
fn missing_or_replaced_external_anchor_permanently_poisons_the_live_instance() {
    for replace in [false, true] {
        let fixture = Fixture::new();
        let mut owner = fixture.initialize();
        let request = Uuid::new_v4();
        let pending = reserve(&mut owner, &fixture.runtime, request);
        let anchor: serde_json::Value = fixture.read(&fixture.anchor());
        fixture.namespace().remove_file(&fixture.anchor()).unwrap();
        if replace {
            fixture.write(&fixture.anchor(), &anchor);
        }
        assert_eq!(
            owner.verify(&fixture.runtime),
            Err(JournalError::Unavailable)
        );
        if !replace {
            fixture.write(&fixture.anchor(), &anchor);
        }
        assert_eq!(
            owner.lookup(&fixture.runtime, request, digest()),
            Err(JournalError::Poisoned)
        );
        assert_eq!(
            owner.complete(&fixture.runtime, pending, Completion::Delivered),
            Err(JournalError::Poisoned)
        );
    }
}

#[test]
fn missing_header_or_history_fails_reopen_without_recreating_the_artifact() {
    for name in [HEADER_FILE, JOURNAL_FILE] {
        let fixture = Fixture::new();
        let owner = fixture.initialize();
        drop(owner);
        let path = fixture.journal().join(name);
        fixture.namespace().remove_file(&path).unwrap();
        assert!(fixture.reopen(&fixture.runtime).is_err());
        assert!(!path.exists());
        assert!(matches!(
            ControlRuntime::initialize_fresh(fixture.proof().unwrap()),
            Err(JournalError::AlreadyExists)
        ));
        assert!(!path.exists());
    }
}

#[test]
fn missing_journal_directory_or_external_anchor_is_never_reinitialized() {
    for remove_anchor in [false, true] {
        let fixture = Fixture::new();
        let owner = fixture.initialize();
        drop(owner);
        let missing = if remove_anchor {
            fixture.anchor()
        } else {
            fixture.journal()
        };
        let moved = fixture
            .paths
            .session_dir(fixture.runtime.terminal_id)
            .join(MOVED);
        std::fs::rename(&missing, &moved).unwrap();
        assert!(fixture.reopen(&fixture.runtime).is_err());
        assert!(!missing.exists());
        assert!(moved.exists());
    }
}

#[test]
fn external_anchor_is_private_and_exclusive_ownership_is_enforced() {
    let fixture = Fixture::new();
    let owner = fixture.initialize();
    let permissions = std::fs::metadata(fixture.anchor()).unwrap().permissions();
    assert_eq!(permissions.mode() & PERMISSION_MASK, PRIVATE_FILE_MODE);
    assert!(matches!(
        fixture.reopen(&fixture.runtime),
        Err(JournalError::Busy)
    ));
    owner.verify(&fixture.runtime).unwrap();
}
