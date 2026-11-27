use super::support::*;

#[test]
fn exhausted_live_ingest_latches_before_event_classification() {
    let fixture = Fixture::new();
    let snapshot = std::fs::read(fixture.runtime.snapshot_path()).unwrap();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    let authority = fixture.runtime.subscribe_authority();
    fixture.runtime.exhaust_active_authority_for_test();

    assert_exhaustion_rejection(fixture.runtime.ingest(event, fixture.process.pid));
    assert_exhausted(&fixture, &authority);
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        fixture.runtime.outbox_path(),
        &fixture.identity,
    )
    .unwrap()
    .load()
    .unwrap();
    assert_eq!(outbox.acknowledged_sequence(), UNASSIGNED_EVENT_SEQUENCE);
    assert_eq!(outbox.pending_for_test().len(), SINGLE_EVENT_COUNT);
    assert_snapshot_unchanged(&fixture, &snapshot);
}

#[test]
fn exhausted_reconcile_latches_with_a_pending_suffix() {
    let fixture = Fixture::new();
    let snapshot = std::fs::read(fixture.runtime.snapshot_path()).unwrap();
    fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    let authority = fixture.runtime.subscribe_authority();
    fixture.runtime.exhaust_active_authority_for_test();

    assert!(fixture.runtime.reconcile_pending().is_err());
    assert_exhausted(&fixture, &authority);
    assert_snapshot_unchanged(&fixture, &snapshot);
}

#[test]
fn exhausted_process_exit_latches_without_terminal_publication() {
    let fixture = Fixture::new();
    let snapshot = std::fs::read(fixture.runtime.snapshot_path()).unwrap();
    let authority = fixture.runtime.subscribe_authority();
    fixture.runtime.exhaust_active_authority_for_test();
    let observed =
        crate::agent::hooks::ProcessIdentity::new(fixture.process.pid, fixture.process.start_time);

    assert!(matches!(
        fixture.runtime.observe_process_exit(observed),
        Err(super::super::error::CodexHookRejection::Violation(
            crate::agent::hooks::HookViolation::AuthorityExhausted
        ))
    ));
    assert_exhausted(&fixture, &authority);
    assert_snapshot_unchanged(&fixture, &snapshot);
}

fn assert_exhaustion_rejection(
    result: Result<
        super::super::runtime::CodexHookPublication,
        super::super::error::CodexHookRejection,
    >,
) {
    assert!(matches!(
        result,
        Err(super::super::error::CodexHookRejection::Violation(
            crate::agent::hooks::HookViolation::AuthorityExhausted
        ))
    ));
}

fn assert_exhausted(
    fixture: &Fixture,
    authority: &tokio::sync::watch::Receiver<super::super::publication::CodexAuthorityView>,
) {
    assert!(!fixture.runtime.storage_failed().unwrap());
    assert!(fixture.runtime.state().unwrap().is_some());
    assert!(matches!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
}

fn assert_snapshot_unchanged(fixture: &Fixture, expected: &[u8]) {
    assert_eq!(
        std::fs::read(fixture.runtime.snapshot_path()).unwrap(),
        expected
    );
}
