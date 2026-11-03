use super::receipts::corrupt_receipt_references;
use super::*;
use crate::claude_hooks::ClaudeHookOperation;

#[test]
fn persisted_authenticated_receipts_must_reference_a_real_session_start() {
    let fixture = Fixture::new();
    fixture.prepare().unwrap();
    fixture.commit().unwrap();
    corrupt_receipt_references(&fixture, true);
}

#[test]
fn persisted_prepared_receipts_must_reference_a_real_session_start() {
    let fixture = Fixture::new();
    fixture.prepare().unwrap();
    corrupt_receipt_references(&fixture, false);
}

#[test]
fn individually_valid_prepared_and_committed_receipts_cannot_name_different_starts() {
    let mut fixture = Fixture::new();
    fixture.prepare().unwrap();
    fixture.commit().unwrap();
    let later = fixture.append_start();
    let mut damaged = fixture.snapshot();
    let queue = &mut damaged.invocations[FIRST_QUEUE];
    assert_ne!(queue.authenticated_session_start.unwrap(), later);
    queue.prepared_session_start.as_mut().unwrap().accepted = later;
    fixture.reject_corruption(
        stringify!(different_prepared_and_committed_starts),
        &damaged,
    );
}

#[test]
fn an_acknowledged_session_start_remains_authenticated_without_a_pending_copy() {
    let fixture = Fixture::new();
    fixture.prepare().unwrap();
    fixture.commit().unwrap();
    let accepted = next(&fixture.path, fixture.delivery).unwrap().unwrap();
    acknowledge(&fixture.path, fixture.delivery, &accepted).unwrap();
    let expected = fixture.bytes();
    let state = fixture.snapshot();
    assert_eq!(
        state.invocations[FIRST_QUEUE].acknowledged_sequence,
        fixture.accepted.event_sequence
    );
    assert!(matches!(
        next(&fixture.path, fixture.delivery)
            .unwrap()
            .unwrap()
            .operation,
        ClaudeHookOperation::Lifecycle {
            event: ClaudeLifecycleEvent::CwdChanged { .. },
            ..
        }
    ));
    assert!(
        fence_session_start_replay(
            &fixture.path,
            fixture.credentials,
            fixture.accepted.event_sequence,
            fixture.accepted.event_id,
            fixture.agent,
            Ok
        )
        .unwrap()
    );
    assert!(
        fence_authenticated_session_start(
            &fixture.path,
            fixture.credentials.invocation_id,
            fixture.agent,
            Ok
        )
        .unwrap()
    );
    assert_eq!(fixture.bytes(), expected);
}
