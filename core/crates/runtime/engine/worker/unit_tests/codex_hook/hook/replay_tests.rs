use super::support::*;
use crate::agent::hooks::{InstrumentationState, TurnState};

const BACKLOG_EVENT_UUID_BASE: u128 = 0x80000000000000000000000000000000;

#[test]
fn missing_outbox_owner_starts_bound_genesis_authority() {
    let fixture = Fixture::new();

    let report = fixture.replay;

    assert_eq!(report.applied, EMPTY_EVENT_COUNT);
    assert_eq!(report.replayed, EMPTY_EVENT_COUNT);
    assert_eq!(report.stale, EMPTY_EVENT_COUNT);
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence.0,
        UNASSIGNED_EVENT_SEQUENCE
    );
}

#[test]
fn startup_replays_pending_suffix_after_acknowledged_snapshot() {
    let fixture = Fixture::new();
    fixture
        .submit(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let prompt = fixture.enqueue(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE));

    let (runtime, report) = fixture.restart_runtime();
    let state = runtime.state().unwrap().unwrap();

    assert_eq!(prompt.sequence.0, SECOND_EVENT_SEQUENCE);
    assert_eq!(report.applied, SINGLE_EVENT_COUNT);
    assert_eq!(report.replayed, EMPTY_EVENT_COUNT);
    assert_eq!(report.stale, EMPTY_EVENT_COUNT);
    assert_eq!(state.last_sequence.0, SECOND_EVENT_SEQUENCE);
    assert_eq!(state.turn, TurnState::Running);
    assert_eq!(state.instrumentation, InstrumentationState::Authoritative);
}

#[test]
fn missing_snapshot_after_acknowledgement_is_rejected_as_rollback() {
    let fixture = Fixture::new();
    fixture
        .submit(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    std::fs::remove_file(fixture.runtime.snapshot_path()).unwrap();

    assert!(fixture.try_restart_runtime().is_err());
}

#[test]
fn snapshot_behind_acknowledged_watermark_is_rejected() {
    let fixture = Fixture::new();
    fixture
        .submit(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let snapshot_path = fixture.runtime.snapshot_path();
    let previous = std::fs::read(&snapshot_path).unwrap();
    fixture
        .submit(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    std::fs::write(snapshot_path, previous).unwrap();

    assert!(fixture.try_restart_runtime().is_err());
}

#[test]
fn replay_rejects_an_outbox_owned_by_another_worker_session() {
    let fixture = Fixture::new();
    let mut other = fixture.identity.clone();
    other.terminal_id = Fixture::another_session();
    let outbox =
        crate::codex_hooks::CodexOutbox::for_identity(fixture.runtime.outbox_path(), &other)
            .unwrap();
    outbox
        .enqueue(envelope(
            other,
            UNASSIGNED_EVENT_SEQUENCE,
            Fixture::other_event_id(),
            crate::agent::hooks::EventKind::SessionStart {
                source: crate::agent::hooks::SessionStartSource::Startup,
            },
        ))
        .unwrap();

    assert!(fixture.try_restart_runtime().is_err());
    assert_eq!(
        fixture.runtime.state().unwrap().unwrap().last_sequence.0,
        UNASSIGNED_EVENT_SEQUENCE
    );
}

#[test]
fn reconciliation_is_idempotent_after_startup_restore() {
    let fixture = Fixture::new();
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        fixture.runtime.outbox_path(),
        &fixture.identity,
    )
    .unwrap();
    outbox
        .enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    let (runtime, report) = fixture.restart_runtime();

    assert_eq!(report.applied, SINGLE_EVENT_COUNT);
    assert_eq!(runtime.reconcile_pending().unwrap(), Default::default());
    assert_eq!(
        runtime.state().unwrap().unwrap().last_sequence.0,
        FIRST_EVENT_SEQUENCE
    );
}

#[test]
fn full_pending_witness_restores_and_old_ack_replays_without_reduction() {
    let fixture = Fixture::new();
    let outbox = crate::codex_hooks::CodexOutbox::for_identity(
        fixture.runtime.outbox_path(),
        &fixture.identity,
    )
    .unwrap();
    let start = outbox
        .enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    fixture
        .runtime
        .ingest(start.clone(), fixture.process.pid)
        .unwrap();
    outbox
        .acknowledge(start.sequence.0, start.event_id)
        .unwrap();

    for index in 0..crate::agent::hooks::MAX_REPLAY_WINDOW {
        outbox.enqueue(backlog_event(&fixture, index)).unwrap();
    }
    assert!(
        outbox
            .enqueue(backlog_event(
                &fixture,
                crate::agent::hooks::MAX_REPLAY_WINDOW
            ))
            .is_err()
    );

    let report = fixture.runtime.reconcile_pending().unwrap();
    assert_eq!(report.applied, crate::agent::hooks::MAX_REPLAY_WINDOW);
    assert_eq!(report.replayed, EMPTY_EVENT_COUNT);
    assert_eq!(report.stale, EMPTY_EVENT_COUNT);

    let (runtime, report) = fixture.restart_runtime();
    assert_eq!(report, Default::default());
    assert_eq!(runtime.reconcile_pending().unwrap(), Default::default());
    let state_before_replay = runtime.state().unwrap().unwrap();
    let replay = runtime.ingest(start, fixture.process.pid).unwrap();
    assert_eq!(
        replay.disposition,
        crate::agent::hooks::ApplyDisposition::Replay
    );
    assert_eq!(replay.state, state_before_replay);
}

fn backlog_event(fixture: &Fixture, index: usize) -> crate::agent::hooks::HookEnvelope {
    let event_id = BACKLOG_EVENT_UUID_BASE
        .checked_add(u128::try_from(index).unwrap())
        .map(uuid::Uuid::from_u128)
        .unwrap();
    envelope(
        fixture.identity.clone(),
        UNASSIGNED_EVENT_SEQUENCE,
        event_id,
        crate::agent::hooks::EventKind::UserPromptSubmit,
    )
}
