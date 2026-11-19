use super::support::*;
use crate::agent::hooks::{ApplyDisposition, HookViolation};
use crate::codex_hooks::{CodexEnqueueAdmission, CodexEnqueueDecision};

const RETAINED_DIRECTORY_EXTENSION: &str = "retained-runtime-authority";

#[test]
fn acknowledged_event_ahead_of_live_snapshot_fails_closed() {
    let fixture = Fixture::new();
    let snapshot = std::fs::read(fixture.runtime.snapshot_path()).unwrap();
    let outbox = configured_outbox(&fixture);
    let event = outbox
        .enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    outbox
        .acknowledge(event.sequence.0, event.event_id)
        .unwrap();

    assert_storage_rejection(fixture.runtime.ingest(event, fixture.process.pid));
    assert_unavailable(&fixture);
    assert_eq!(
        std::fs::read(fixture.runtime.snapshot_path()).unwrap(),
        snapshot
    );
}

#[test]
fn retained_replay_rejects_a_replacement_invocation_directory() {
    let fixture = Fixture::new();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    fixture.submit(event.clone()).unwrap();
    replace_invocation_directory(&fixture);

    assert_authority_lost(fixture.runtime.ingest(event, fixture.process.pid));
    assert_authority_lost_view(&fixture);
}

#[test]
fn empty_reconcile_rejects_a_replacement_invocation_directory() {
    let fixture = Fixture::new();
    fixture.submit(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap();
    replace_invocation_directory(&fixture);

    let error = fixture.runtime.reconcile_pending().unwrap_err();
    assert!(super::super::authority::is_authority_lost_error(&error));
    assert_authority_lost_view(&fixture);
}

#[test]
fn committed_pending_prefix_replays_after_later_event_revokes_authority() {
    let fixture = Fixture::new();
    let outbox = configured_outbox(&fixture);
    let start = outbox
        .enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let mut invalid = fixture.start(UNASSIGNED_EVENT_SEQUENCE);
    invalid.event_id = uuid::Uuid::nil();
    outbox
        .enqueue_admitted_decision(
            invalid,
            None,
            |_, admission| {
                assert_eq!(admission, CodexEnqueueAdmission::New);
                Ok(CodexEnqueueDecision::rejected(
                    (),
                    HookViolation::InvalidTopology,
                ))
            },
            |_, _, _| Ok(()),
        )
        .unwrap();

    let report = fixture.runtime.reconcile_pending().unwrap();
    let before = fixture.runtime.state().unwrap().unwrap();
    let snapshot = std::fs::read(fixture.runtime.snapshot_path()).unwrap();
    let replay = fixture.runtime.ingest(start, fixture.process.pid).unwrap();

    assert_eq!(report.applied, SINGLE_EVENT_COUNT);
    assert!(before.revoked);
    assert_eq!(replay.disposition, ApplyDisposition::Replay);
    assert_eq!(replay.state, before);
    assert_eq!(
        std::fs::read(fixture.runtime.snapshot_path()).unwrap(),
        snapshot
    );
}

fn configured_outbox(fixture: &Fixture) -> crate::codex_hooks::CodexOutbox {
    crate::codex_hooks::CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity)
        .unwrap()
}

fn replace_invocation_directory(fixture: &Fixture) {
    let outbox = fixture.runtime.outbox_path();
    let snapshot = fixture.runtime.snapshot_path();
    let directory = outbox.parent().unwrap();
    let retained = directory.with_extension(RETAINED_DIRECTORY_EXTENSION);
    std::fs::rename(directory, &retained).unwrap();
    crate::paths::ensure_private_parent(&outbox).unwrap();
    copy_artifact(&retained, &outbox);
    copy_artifact(&retained, &snapshot);
}

fn copy_artifact(retained: &std::path::Path, target: &std::path::Path) {
    let source = retained.join(target.file_name().unwrap());
    std::fs::copy(source, target).unwrap();
}

fn assert_storage_rejection(
    result: Result<
        super::super::runtime::CodexHookPublication,
        super::super::error::CodexHookRejection,
    >,
) {
    assert!(matches!(
        result,
        Err(super::super::error::CodexHookRejection::Storage(_))
    ));
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

fn assert_unavailable(fixture: &Fixture) {
    assert!(fixture.runtime.storage_failed().unwrap());
    assert!(fixture.runtime.state().unwrap().is_none());
    assert_eq!(
        fixture.runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::StorageUnavailable
    );
}

fn assert_authority_lost_view(fixture: &Fixture) {
    assert!(!fixture.runtime.storage_failed().unwrap());
    assert!(fixture.runtime.state().is_err());
    assert_eq!(
        fixture.runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
}
