use super::exhaustion_support::*;
use super::exhaustion_tests::final_event;
use super::support::*;
use std::sync::Arc;

#[tokio::test]
async fn final_revision_handler_returns_correlated_revocation() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let event = enqueue_final(&fixture, &runtime);

    let (ack, publication) = exchange_uncommitted(Arc::clone(&runtime), &event).await;

    assert!(!ack.accepted);
    assert_eq!(ack.code, super::super::constants::CODEX_ACK_REVOKED);
    assert_eq!(ack.event_id, Some(event.event_id));
    assert_eq!(ack.sequence, Some(event.sequence.0));
    assert!(publication.is_none());
    assert!(matches!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::Exhausted(_)
    ));
}

#[tokio::test]
async fn competing_final_head_clients_are_both_revoked() {
    let fixture = Fixture::new();
    install_penultimate_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    let event = enqueue_final(&fixture, &runtime);
    let gate = runtime.gate_next_handoff_for_test();
    let first = spawn_exchange(Arc::clone(&runtime), event.clone());
    let entered = Arc::clone(&gate);
    tokio::task::spawn_blocking(move || entered.wait_until_entered())
        .await
        .unwrap();
    let second = spawn_exchange(Arc::clone(&runtime), event.clone());
    assert!(!second.is_finished());

    gate.release();
    let (first_ack, first_publication) = first.await.unwrap();
    let (second_ack, second_publication) = second.await.unwrap();

    for ack in [first_ack, second_ack] {
        assert!(!ack.accepted);
        assert_eq!(ack.code, super::super::constants::CODEX_ACK_REVOKED);
        assert_eq!(ack.sequence, Some(event.sequence.0));
    }
    assert!(first_publication.is_none());
    assert!(second_publication.is_none());
    let outbox =
        crate::codex_hooks::CodexOutbox::for_identity(runtime.outbox_path(), &fixture.identity)
            .unwrap()
            .load()
            .unwrap();
    assert_eq!(
        outbox.acknowledged_sequence(),
        event.sequence.0 - FIRST_EVENT_SEQUENCE
    );
    assert_eq!(outbox.pending_for_test().front(), Some(&event));
}

fn enqueue_final(
    fixture: &Fixture,
    runtime: &super::super::runtime::CodexHookRuntime,
) -> crate::agent::hooks::HookEnvelope {
    crate::codex_hooks::CodexOutbox::for_identity(runtime.outbox_path(), &fixture.identity)
        .unwrap()
        .enqueue(final_event(fixture, UNASSIGNED_EVENT_SEQUENCE))
        .unwrap()
}

fn spawn_exchange(
    runtime: Arc<super::super::runtime::CodexHookRuntime>,
    event: crate::agent::hooks::HookEnvelope,
) -> tokio::task::JoinHandle<(
    super::super::protocol::CodexHookAck,
    Option<crate::agent::hooks::ApplyDisposition>,
)> {
    tokio::spawn(async move { exchange_uncommitted(runtime, &event).await })
}
