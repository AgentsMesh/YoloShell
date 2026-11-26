use super::super::constants::{
    FIRST_SEQUENCE, MAX_ACKNOWLEDGED_EVENTS, MAX_PENDING_EVENTS, OUTBOX_FILE_NAME,
};
use super::{envelope, identity};
use crate::agent::hooks::{EventKind, EventSequence, SessionStartSource};
use crate::codex_hooks::{CodexOutbox, CodexOutboxState};
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

const SECOND_SEQUENCE: u64 = FIRST_SEQUENCE + FIRST_SEQUENCE;

fn retained_limit() -> u64 {
    u64::try_from(MAX_ACKNOWLEDGED_EVENTS).unwrap()
}

#[test]
fn pending_events_fit_the_reducer_replay_witness() {
    assert!(MAX_PENDING_EVENTS <= crate::agent::hooks::MAX_REPLAY_WINDOW);
}

fn consecutive(first: u64, last: u64) -> Vec<u64> {
    (first..=last).collect()
}

fn acknowledged_state(acknowledged_sequence: u64, sequences: &[u64]) -> CodexOutboxState {
    let owner = identity();
    let mut state = CodexOutboxState {
        acknowledged_sequence,
        next_sequence: acknowledged_sequence.checked_add(FIRST_SEQUENCE).unwrap(),
        owner: Some(owner.hook.clone()),
        ..CodexOutboxState::default()
    };
    for sequence in sequences {
        let mut event = envelope(&owner, Uuid::new_v4());
        event.sequence = EventSequence(*sequence);
        state.acknowledged_events.push_back(event);
    }
    state
}

fn persist(root: &TempDir, state: &CodexOutboxState) -> PathBuf {
    let path = root.path().join(OUTBOX_FILE_NAME);
    crate::paths::atomic_write_json(&path, state).unwrap();
    path
}

fn assert_rejected(state: CodexOutboxState) {
    let root = tempfile::tempdir().unwrap();
    assert!(CodexOutbox::new(persist(&root, &state)).is_err());
}

fn assert_accepted(root: &TempDir, state: CodexOutboxState) {
    let expected = state.clone();
    let outbox = CodexOutbox::new(persist(root, &state)).unwrap();
    assert_eq!(outbox.load().unwrap(), expected);
}

#[test]
fn rejects_a_hole_inside_the_retained_acknowledged_tail() {
    let acknowledged = retained_limit().checked_add(FIRST_SEQUENCE).unwrap();
    let mut sequences = consecutive(FIRST_SEQUENCE, acknowledged);
    let missing = FIRST_SEQUENCE.checked_add(FIRST_SEQUENCE).unwrap();
    sequences.retain(|sequence| *sequence != missing);

    assert_rejected(acknowledged_state(acknowledged, &sequences));
}

#[test]
fn rejects_a_short_acknowledged_history_below_the_retention_limit() {
    let acknowledged = FIRST_SEQUENCE.checked_add(FIRST_SEQUENCE).unwrap();

    assert_rejected(acknowledged_state(acknowledged, &[acknowledged]));
}

#[test]
fn rejects_a_short_acknowledged_tail_at_the_retention_boundary() {
    let acknowledged = retained_limit().checked_add(FIRST_SEQUENCE).unwrap();
    let first = acknowledged
        .checked_sub(retained_limit())
        .and_then(|value| value.checked_add(FIRST_SEQUENCE))
        .unwrap();
    let mut sequences = consecutive(first, acknowledged);
    sequences.retain(|sequence| *sequence != first);

    assert_rejected(acknowledged_state(acknowledged, &sequences));
}

#[test]
fn accepts_exact_acknowledged_tails_at_both_retention_boundaries() {
    let root = tempfile::tempdir().unwrap();
    let at_limit = retained_limit();
    assert_accepted(
        &root,
        acknowledged_state(at_limit, &consecutive(FIRST_SEQUENCE, at_limit)),
    );

    let root = tempfile::tempdir().unwrap();
    let past_limit = at_limit.checked_add(FIRST_SEQUENCE).unwrap();
    let first_retained = past_limit
        .checked_sub(at_limit)
        .and_then(|value| value.checked_add(FIRST_SEQUENCE))
        .unwrap();
    assert_accepted(
        &root,
        acknowledged_state(past_limit, &consecutive(first_retained, past_limit)),
    );
}

#[test]
fn accepts_an_empty_history_at_the_zero_waterline() {
    let root = tempfile::tempdir().unwrap();
    assert_accepted(&root, CodexOutboxState::default());
}

#[test]
fn identical_unassigned_compact_occurrences_are_both_retained() {
    let root = tempfile::tempdir().unwrap();
    let owner = identity();
    let outbox =
        CodexOutbox::for_identity(root.path().join(OUTBOX_FILE_NAME), &owner.hook).unwrap();
    let mut compact = envelope(&owner, Uuid::nil());
    compact.turn_id = None;
    compact.event = EventKind::SessionStart {
        source: SessionStartSource::Compact,
    };

    let first = outbox.enqueue(compact.clone()).unwrap();
    let second = outbox.enqueue(compact).unwrap();

    assert_eq!(first.sequence.0, FIRST_SEQUENCE);
    assert_eq!(second.sequence.0, SECOND_SEQUENCE);
    assert!(!first.event_id.is_nil());
    assert_ne!(first.event_id, second.event_id);
    assert_eq!(outbox.pending().unwrap(), vec![first, second]);
}

#[test]
fn persisted_nil_event_identity_is_rejected() {
    let owner = identity();
    let mut pending = CodexOutboxState {
        next_sequence: SECOND_SEQUENCE,
        owner: Some(owner.hook.clone()),
        ..CodexOutboxState::default()
    };
    let mut event = envelope(&owner, Uuid::nil());
    event.sequence = EventSequence::FIRST;
    pending.pending.push_back(event);
    assert_rejected(pending);

    let mut acknowledged = acknowledged_state(FIRST_SEQUENCE, &[FIRST_SEQUENCE]);
    acknowledged
        .acknowledged_events
        .front_mut()
        .unwrap()
        .event_id = Uuid::nil();
    assert_rejected(acknowledged);
}
