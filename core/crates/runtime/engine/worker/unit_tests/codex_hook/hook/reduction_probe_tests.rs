use super::support::{Fixture, UNASSIGNED_EVENT_SEQUENCE};
use crate::agent::hooks::{HookEnvelope, HookViolation};
use crate::codex_hooks::{CodexDurableRejection, CodexOutbox};

#[test]
fn live_unmarked_durable_rejection_does_not_mutate_snapshot() {
    let fixture = Fixture::new();
    let event = enqueue_before_session_start(&fixture);
    let before_snapshot = read_snapshot(&fixture);

    assert_storage_rejection(fixture.runtime.ingest(event, fixture.process.pid));

    assert_eq!(read_snapshot(&fixture), before_snapshot);
}

#[test]
fn recovery_mismatched_rejection_marker_does_not_mutate_snapshot() {
    let fixture = Fixture::new();
    let event = enqueue_before_session_start(&fixture);
    let outbox = outbox(&fixture);
    let mut state = outbox.load().unwrap();
    state.durable_rejections_mut_for_test().push_back(
        CodexDurableRejection::capture_for_test(&event, HookViolation::SessionMismatch).unwrap(),
    );
    crate::paths::atomic_write_json(outbox.path(), &state).unwrap();
    let before_snapshot = read_snapshot(&fixture);

    assert!(fixture.try_restart_runtime().is_err());

    assert_eq!(read_snapshot(&fixture), before_snapshot);
}

fn enqueue_before_session_start(fixture: &Fixture) -> HookEnvelope {
    outbox(fixture)
        .enqueue(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap()
}

fn outbox(fixture: &Fixture) -> CodexOutbox {
    CodexOutbox::for_identity(fixture.runtime.outbox_path(), &fixture.identity).unwrap()
}

fn read_snapshot(fixture: &Fixture) -> Vec<u8> {
    std::fs::read(fixture.runtime.snapshot_path()).unwrap()
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
