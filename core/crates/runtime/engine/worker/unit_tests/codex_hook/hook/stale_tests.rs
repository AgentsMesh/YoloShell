use super::super::protocol::CodexHookAck;
use super::support::*;
use crate::agent::hooks::{
    ApplyDisposition, EventKind, HookEnvelope, SessionEndReason, TerminationCandidate, TurnState,
};
use std::sync::Arc;
use uuid::Uuid;

const THIRD_EVENT_SEQUENCE: u64 = SECOND_EVENT_SEQUENCE + crate::agent::hooks::FIRST_SEQUENCE;
const TERMINAL_EVENT_UUID: u128 = 0x65000000000000000000000000000006;
const APPLIED_START_AND_END: usize = SINGLE_EVENT_COUNT + SINGLE_EVENT_COUNT;

#[tokio::test]
async fn stale_event_receives_accepted_ack_and_exact_replay_ack() {
    let fixture = Fixture::new();
    let start = fixture.start(FIRST_EVENT_SEQUENCE);
    exchange(Arc::clone(&fixture.runtime), &start).await;
    let terminal = end(&fixture, SECOND_EVENT_SEQUENCE);
    exchange(Arc::clone(&fixture.runtime), &terminal).await;
    let stale = fixture.prompt(THIRD_EVENT_SEQUENCE);

    let (accepted, publication) = exchange(Arc::clone(&fixture.runtime), &stale).await;
    assert_success_ack(&accepted, &stale, crate::codex_hooks::ACK_CODE_ACCEPTED);
    assert_eq!(publication, Some(ApplyDisposition::Stale));
    let stale_state = fixture.runtime.state().unwrap().unwrap();

    let (replay, publication) = exchange(Arc::clone(&fixture.runtime), &stale).await;
    assert_success_ack(&replay, &stale, crate::codex_hooks::ACK_CODE_REPLAY);
    assert_eq!(publication, Some(ApplyDisposition::Replay));
    assert_eq!(fixture.runtime.state().unwrap().unwrap(), stale_state);
}

#[test]
fn startup_replay_counts_stale_events_and_preserves_terminal_state() {
    let fixture = Fixture::new();
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        fixture.runtime.outbox_path(),
        &fixture.identity,
    )
    .unwrap();
    outbox
        .enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    outbox
        .enqueue(end(&fixture, UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    outbox
        .enqueue(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();

    let (runtime, report) = fixture.restart_runtime();
    let state = runtime.state().unwrap().unwrap();

    assert_eq!(report.applied, APPLIED_START_AND_END);
    assert_eq!(report.replayed, EMPTY_EVENT_COUNT);
    assert_eq!(report.stale, SINGLE_EVENT_COUNT);
    assert_eq!(state.last_sequence.0, THIRD_EVENT_SEQUENCE);
    assert_eq!(state.turn, TurnState::Unknown);
    assert_eq!(
        state.termination_candidate,
        Some(TerminationCandidate::Final)
    );
    assert!(state.final_hook_seen);
}

fn end(fixture: &Fixture, sequence: u64) -> HookEnvelope {
    envelope(
        fixture.identity.clone(),
        sequence,
        Uuid::from_u128(TERMINAL_EVENT_UUID),
        EventKind::SessionEnd {
            reason: SessionEndReason::Other,
        },
    )
}

fn assert_success_ack(ack: &CodexHookAck, event: &HookEnvelope, code: &str) {
    assert_eq!(ack.schema_version, crate::codex_hooks::ACK_SCHEMA_VERSION);
    assert!(ack.accepted);
    assert_eq!(ack.event_id, Some(event.event_id));
    assert_eq!(ack.sequence, Some(event.sequence.0));
    assert_eq!(ack.code, code);
}
