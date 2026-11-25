use super::support::*;
use crate::codex_hooks::{
    CodexSnapshotCursor, OutboxPostOperationFault, fail_snapshot_after_commit,
    fail_snapshot_before_commit, inject_outbox_fault,
};

#[test]
fn snapshot_lock_loss_before_commit_latches_without_installing_state() {
    let fixture = Fixture::new();
    let snapshot_before = std::fs::read(fixture.runtime.snapshot_path()).unwrap();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    let outbox_before = std::fs::read(fixture.runtime.outbox_path()).unwrap();
    fail_snapshot_before_commit(&fixture.runtime.snapshot_path());

    assert_authority_lost(fixture.runtime.ingest(event.clone(), fixture.process.pid));
    assert_authority_lost_latched(&fixture);
    assert_eq!(
        std::fs::read(fixture.runtime.snapshot_path()).unwrap(),
        snapshot_before
    );

    assert_authority_lost(fixture.runtime.ingest(event, fixture.process.pid));
    assert_eq!(
        std::fs::read(fixture.runtime.outbox_path()).unwrap(),
        outbox_before
    );
    assert_eq!(
        std::fs::read(fixture.runtime.snapshot_path()).unwrap(),
        snapshot_before
    );
}

#[test]
fn snapshot_lock_loss_after_commit_latches_and_restart_recovers() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    fail_snapshot_after_commit(&fixture.runtime.snapshot_path());

    assert_authority_lost(fixture.runtime.ingest(event, fixture.process.pid));
    assert_authority_lost_latched(&fixture);
    assert!(fixture.runtime.snapshot_path().exists());

    let (runtime, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence.0,
        FIRST_EVENT_SEQUENCE
    );
}

#[test]
fn stale_snapshot_cursor_loses_authority_before_overwriting_newer_authority() {
    let fixture = Fixture::new();
    let snapshot_path = fixture.runtime.snapshot_path();
    let (external, installed) = CodexSnapshotCursor::open(&snapshot_path).unwrap();
    external.persist(&installed.unwrap()).unwrap();
    let external_bytes = std::fs::read(&snapshot_path).unwrap();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));

    assert!(matches!(
        fixture.runtime.ingest(event, fixture.process.pid),
        Err(super::super::error::CodexHookRejection::AuthorityLost(_))
    ));
    assert!(!fixture.runtime.storage_failed().unwrap());
    assert!(fixture.runtime.state().is_err());
    assert_eq!(
        fixture.runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
    assert_eq!(std::fs::read(snapshot_path).unwrap(), external_bytes);
}

#[test]
fn outbox_post_operation_substitution_never_publishes_authority() {
    let faults = [
        OutboxPostOperationFault::TargetRemoved,
        OutboxPostOperationFault::LockRemoved,
        OutboxPostOperationFault::DirectoryMoved,
    ];
    for fault in faults {
        let fixture = Fixture::new();
        let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
        inject_outbox_fault(&fixture.runtime.outbox_path(), fault);

        let result = fixture.runtime.ingest(event, fixture.process.pid);
        assert_outbox_fault_contained(&fixture, fault, result);
    }
}

#[test]
fn replay_does_not_rotate_the_snapshot_cursor() {
    let fixture = Fixture::new();
    let event = fixture.start(UNASSIGNED_EVENT_SEQUENCE);
    fixture.submit(event.clone()).unwrap();
    let (external, snapshot) = CodexSnapshotCursor::open(fixture.runtime.snapshot_path()).unwrap();

    let replay = fixture.submit(event).unwrap();

    assert_eq!(
        replay.disposition,
        crate::agent::hooks::ApplyDisposition::Replay
    );
    assert!(external.persist(&snapshot.unwrap()).is_ok());
}

fn assert_authority_lost(
    result: Result<
        super::super::runtime::CodexHookPublication,
        super::super::error::CodexHookRejection,
    >,
) {
    assert!(matches!(
        result,
        Err(super::super::error::CodexHookRejection::AuthorityLost(_))
    ));
}

fn assert_authority_lost_latched(fixture: &Fixture) {
    assert!(!fixture.runtime.storage_failed().unwrap());
    assert!(fixture.runtime.state().is_err());
    assert_eq!(
        fixture.runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
}

fn assert_outbox_fault_contained(
    fixture: &Fixture,
    fault: OutboxPostOperationFault,
    result: Result<
        super::super::runtime::CodexHookPublication,
        super::super::error::CodexHookRejection,
    >,
) {
    match fault {
        OutboxPostOperationFault::DirectoryMoved => {
            assert_authority_lost(result);
            assert_authority_lost_latched(fixture);
        }
        OutboxPostOperationFault::TargetRemoved | OutboxPostOperationFault::LockRemoved => {
            assert!(matches!(
                result,
                Err(super::super::error::CodexHookRejection::Storage(_))
            ));
            assert!(fixture.runtime.storage_failed().unwrap());
            assert_eq!(
                fixture.runtime.subscribe_authority().borrow().clone(),
                super::super::publication::CodexAuthorityView::StorageUnavailable
            );
        }
    }
}
