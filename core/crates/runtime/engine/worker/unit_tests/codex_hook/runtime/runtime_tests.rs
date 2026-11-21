use super::super::constants::{CODEX_ACK_IDENTITY, CODEX_ACK_PROCESS, CODEX_ACK_PROTOCOL};
use super::support::*;
use crate::agent::hooks::{
    ApplyDisposition, InstrumentationState, ProcessState, HookProviderKind, TurnState,
};

const UNOBSERVED_PEER_PID: u32 = u32::MAX;
const THIRD_EVENT_SEQUENCE: u64 = SECOND_EVENT_SEQUENCE + crate::agent::hooks::FIRST_SEQUENCE;
const END_EVENT_UUID: u128 = 0x65000000000000000000000000000006;

#[test]
fn first_session_start_publishes_authoritative_idle_state() {
    let fixture = Fixture::new();

    let publication = fixture.submit(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap();

    assert_eq!(publication.disposition, ApplyDisposition::Applied);
    assert_eq!(publication.state.process, ProcessState::Alive);
    assert_eq!(publication.state.turn, TurnState::Idle);
    assert_eq!(
        publication.state.instrumentation,
        InstrumentationState::Authoritative
    );
    assert_eq!(
        publication.state.provider_session_id.as_deref(),
        Some(TEST_PROVIDER_SESSION)
    );
}

#[test]
fn duplicate_event_is_an_idempotent_replay() {
    let fixture = Fixture::new();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    fixture.submit(event.clone()).unwrap();

    let publication = fixture.submit(event).unwrap();

    assert_eq!(publication.disposition, ApplyDisposition::Replay);
    assert_eq!(publication.state.last_sequence.0, FIRST_EVENT_SEQUENCE);
}

#[test]
fn late_event_after_session_end_is_acknowledged_as_stale_without_mutation() {
    let fixture = Fixture::new();
    fixture.submit(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap();
    let ended = fixture.submit(end(&fixture)).unwrap();

    let publication = fixture
        .submit(fixture.prompt(THIRD_EVENT_SEQUENCE))
        .unwrap();

    assert_eq!(publication.disposition, ApplyDisposition::Stale);
    assert_eq!(publication.state.turn, ended.state.turn);
    assert_eq!(
        publication.state.termination_candidate,
        ended.state.termination_candidate
    );
    assert_eq!(
        publication.state.final_hook_seen,
        ended.state.final_hook_seen
    );
}

fn end(fixture: &Fixture) -> crate::agent::hooks::HookEnvelope {
    envelope(
        fixture.identity.clone(),
        SECOND_EVENT_SEQUENCE,
        uuid::Uuid::from_u128(END_EVENT_UUID),
        crate::agent::hooks::EventKind::SessionEnd {
            reason: crate::agent::hooks::SessionEndReason::Other,
        },
    )
}

#[test]
fn another_worker_session_cannot_claim_authority() {
    let fixture = Fixture::new();
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.identity.terminal_id = Fixture::another_session();

    let rejection = fixture
        .runtime
        .ingest(event, fixture.process.pid)
        .unwrap_err();

    assert_eq!(rejection.code(), CODEX_ACK_IDENTITY);
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence.0,
        UNASSIGNED_EVENT_SEQUENCE
    );
}

#[test]
fn unavailable_process_identity_is_rejected_before_reduction() {
    let fixture = Fixture::new();
    let mut event = fixture.start(FIRST_EVENT_SEQUENCE);
    event.identity.process.start_time = event
        .identity
        .process
        .start_time
        .saturating_add(INVALID_PROCESS_START_DELTA);

    let rejection = fixture
        .runtime
        .ingest(event, fixture.process.pid)
        .unwrap_err();

    assert_eq!(rejection.code(), CODEX_ACK_IDENTITY);
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence.0,
        UNASSIGNED_EVENT_SEQUENCE
    );
}

#[test]
fn unobserved_same_user_peer_cannot_submit_an_agent_event() {
    let fixture = Fixture::new();

    let rejection = fixture
        .runtime
        .ingest(fixture.start(FIRST_EVENT_SEQUENCE), UNOBSERVED_PEER_PID)
        .unwrap_err();

    assert_eq!(rejection.code(), CODEX_ACK_PROCESS);
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence.0,
        UNASSIGNED_EVENT_SEQUENCE
    );
}

#[test]
fn undurable_socket_event_cannot_revoke_authority() {
    let fixture = Fixture::new();
    fixture.submit(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap();

    let gap = fixture
        .runtime
        .ingest(fixture.prompt(GAP_EVENT_SEQUENCE), fixture.process.pid)
        .unwrap_err();
    let after = fixture
        .submit(fixture.prompt(SECOND_EVENT_SEQUENCE))
        .unwrap();

    assert_eq!(gap.code(), CODEX_ACK_PROTOCOL);
    assert_eq!(after.disposition, ApplyDisposition::Applied);
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().instrumentation,
        InstrumentationState::Authoritative
    );
}

#[test]
fn invalid_first_envelope_is_rejected_before_claiming_authority() {
    let fixture = Fixture::new();
    let authority = fixture.runtime.subscribe_authority();
    let before = fixture.runtime.state().unwrap().unwrap();
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        fixture.runtime.outbox_path(),
        &fixture.identity,
    )
    .unwrap();
    let mut invalid = fixture.start(FIRST_EVENT_SEQUENCE);
    invalid.provider = HookProviderKind::Claude;

    assert!(outbox.enqueue(invalid.clone()).is_err());
    assert!(outbox.enqueue(invalid).is_err());

    assert!(outbox.pending().unwrap().is_empty());
    assert!(fixture.runtime.snapshot_path().exists());
    assert_eq!(fixture.runtime.state().unwrap().unwrap(), before);
    assert!(matches!(
        authority.borrow().clone(),
        super::super::publication::CodexAuthorityView::Active(state) if !state.revoked
    ));
}
