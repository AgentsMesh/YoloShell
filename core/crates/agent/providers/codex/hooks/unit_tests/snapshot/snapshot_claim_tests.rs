use super::snapshot_test_support::*;
use crate::codex_hooks::snapshot_error::CodexSnapshotFenceError;
use crate::codex_hooks::{
    CodexSnapshotCursor, SnapshotClaimFault, SnapshotClaimStage, inject_snapshot_claim_fault,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[test]
fn missing_cursor_claim_publishes_canonical_initial_and_advances_cursor() {
    let fixture = SnapshotFixture::new();
    let initial = snapshot();
    let expected = encoded(&initial);
    let (mut cursor, loaded) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    assert!(loaded.is_none());

    let token = publish_claim(&mut cursor, &initial);

    let actual: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&fixture.path).unwrap()).unwrap();
    assert_eq!(actual, expected);
    assert!(cursor.is_claimed_by(&token));
}

#[test]
fn existing_claim_rewrites_disk_value_unchanged_with_a_fresh_identity() {
    let (fixture, mut cursor) = SnapshotFixture::populated();
    let before = std::fs::read(&fixture.path).unwrap();
    let before_value: serde_json::Value = serde_json::from_slice(&before).unwrap();
    let ignored_initial = snapshot();
    assert_ne!(encoded(&ignored_initial), before_value);

    let token = publish_claim(&mut cursor, &ignored_initial);

    assert_eq!(std::fs::read(&fixture.path).unwrap(), before);
    assert!(cursor.is_claimed_by(&token));
}

#[test]
fn successful_claim_revokes_every_other_cursor_for_the_old_snapshot() {
    let (fixture, mut cursor) = SnapshotFixture::populated();
    let (stale, _) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    publish_claim(&mut cursor, &snapshot());

    assert_snapshot_changed(stale.prove_current().err().unwrap());
    assert_snapshot_changed(stale.persist(&snapshot()).err().unwrap());
    assert!(fixture.path.exists());
}

#[test]
fn only_first_cursor_with_the_same_expected_identity_can_claim() {
    let (fixture, mut first) = SnapshotFixture::populated();
    let (mut second, _) = CodexSnapshotCursor::open(&fixture.path).unwrap();
    let (mut third, _) = CodexSnapshotCursor::open(&fixture.path).unwrap();

    let token = publish_claim(&mut first, &snapshot());
    assert!(first.is_claimed_by(&token));

    assert_snapshot_changed(second.claim(&snapshot()).err().unwrap());
    assert_snapshot_changed(third.claim(&snapshot()).err().unwrap());
}

#[test]
fn claim_commit_fence_classifies_each_replaced_authority() {
    for fault in authority_faults() {
        let (fixture, mut cursor) = SnapshotFixture::populated();
        inject_snapshot_claim_fault(&fixture.path, SnapshotClaimStage::BeforeCommit, fault);

        assert_fault(cursor.claim(&snapshot()).err().unwrap(), fault);
    }
}

#[test]
fn claim_post_rename_verification_classifies_each_replaced_authority() {
    for fault in authority_faults() {
        let (fixture, mut cursor) = SnapshotFixture::populated();
        inject_snapshot_claim_fault(&fixture.path, SnapshotClaimStage::AfterRename, fault);

        assert_fault(cursor.claim(&snapshot()).err().unwrap(), fault);
    }
}

#[test]
fn post_rename_observation_never_mints_a_claim_after_write_failure() {
    let (fixture, mut cursor) = SnapshotFixture::populated();
    let observed = Arc::new(AtomicBool::new(false));
    let callback_observed = Arc::clone(&observed);
    let path = fixture.path.clone();
    super::super::snapshot_claim::set_after_claim_rename_hook(move || {
        callback_observed.store(true, Ordering::SeqCst);
        crate::codex_hooks::replace_snapshot_same_content(&path);
    });

    let result = cursor.claim(&snapshot());

    assert!(observed.load(Ordering::SeqCst));
    assert_snapshot_changed(result.err().unwrap());
}

#[test]
fn uncertain_post_commit_outcome_is_indeterminate() {
    let (fixture, mut cursor) = SnapshotFixture::populated();
    inject_snapshot_claim_fault(
        &fixture.path,
        SnapshotClaimStage::AfterRename,
        SnapshotClaimFault::TargetHardLinked,
    );

    let error = cursor.claim(&snapshot()).err().unwrap();

    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::Indeterminate(_))
    ));
}

#[test]
fn retained_fresh_target_after_observed_write_error_is_indeterminate() {
    let (fixture, mut cursor) = SnapshotFixture::populated();
    let observed = Arc::new(AtomicBool::new(false));
    let callback_observed = Arc::clone(&observed);
    super::super::snapshot_claim::set_after_claim_rename_hook(move || {
        callback_observed.store(true, Ordering::SeqCst);
    });
    let entry = crate::paths::private_file::PrivateFileEntry::open(&fixture.path).unwrap();
    let _failure = crate::paths::test_support::install_after_move_sync_hook(&entry, || {
        Err(anyhow::anyhow!(
            crate::codex_hooks::constants::errors::SNAPSHOT_CORRUPT
        ))
    });

    let error = cursor.claim(&snapshot()).err().unwrap();

    assert!(observed.load(Ordering::SeqCst));
    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::Indeterminate(_))
    ));
    assert_snapshot_changed(cursor.prove_current().err().unwrap());
}

fn publish_claim(
    cursor: &mut CodexSnapshotCursor,
    snapshot: &crate::agent::hooks::ReducerSnapshot,
) -> crate::codex_hooks::CodexSnapshotClaimToken {
    cursor
        .claim(snapshot)
        .unwrap()
        .publish_with(|proof| proof.linearize_publication())
        .unwrap()
}

fn authority_faults() -> impl Iterator<Item = SnapshotClaimFault> {
    [
        SnapshotClaimFault::TargetReplaced,
        SnapshotClaimFault::LockReplaced,
        SnapshotClaimFault::DirectoryReplaced,
    ]
    .into_iter()
}

fn assert_fault(error: anyhow::Error, fault: SnapshotClaimFault) {
    let actual = error.downcast_ref::<CodexSnapshotFenceError>();
    assert!(match fault {
        SnapshotClaimFault::TargetReplaced => {
            matches!(actual, Some(CodexSnapshotFenceError::SnapshotChanged))
        }
        SnapshotClaimFault::LockReplaced => {
            matches!(actual, Some(CodexSnapshotFenceError::LockPathChanged))
        }
        SnapshotClaimFault::DirectoryReplaced => matches!(
            actual,
            Some(CodexSnapshotFenceError::InvocationDirectoryChanged)
        ),
        SnapshotClaimFault::TargetHardLinked => {
            matches!(actual, Some(CodexSnapshotFenceError::Indeterminate(_)))
        }
    });
}

fn assert_snapshot_changed(error: anyhow::Error) {
    assert!(matches!(
        error.downcast_ref::<CodexSnapshotFenceError>(),
        Some(CodexSnapshotFenceError::SnapshotChanged)
    ));
}
