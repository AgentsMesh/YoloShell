use super::*;
use crate::test_support::{CLAUDE_VERSION, claude_envelope, claude_started, identity};
use crate::{EventKind, HookProviderKind, TurnState};

const STEP: u64 = 1;

fn reducer() -> Reducer {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    claude_started(&mut reducer, &identity);
    let event = claude_envelope(
        &identity,
        crate::FIRST_SEQUENCE + STEP,
        EventKind::UserPromptSubmit,
    );
    reducer.apply(event).unwrap();
    reducer.snapshot().unwrap();
    reducer
}

#[test]
fn history_indexes_require_matching_cardinality_event_keys_continuity_and_tail() {
    let original = reducer();
    let mutations: &[(&str, fn(&mut NormalizedLifecycle))] = &[
        (stringify!(event_ids), |value| {
            value.event_ids.pop_first();
        }),
        (stringify!(invalid_envelope), |value| {
            value.history.first_entry().unwrap().get_mut().event_id = uuid::Uuid::nil()
        }),
        (stringify!(entry_sequence), |value| {
            value.history.first_entry().unwrap().get_mut().sequence = EventSequence(u64::MAX)
        }),
        (stringify!(sequence_gap), |value| {
            let (_, mut event) = value.history.pop_last().unwrap();
            event.sequence.0 += STEP;
            value.event_ids.insert(event.event_id, event.sequence);
            value.history.insert(event.sequence, event);
            value.last_sequence.0 += STEP;
        }),
        (stringify!(event_index), |value| {
            *value.event_ids.first_entry().unwrap().get_mut() = EventSequence(u64::MAX);
        }),
        (stringify!(tail), |value| value.last_sequence.0 += STEP),
        (stringify!(missing_history), |value| {
            value.history.clear();
            value.event_ids.clear();
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.state().clone();
        mutate(&mut changed);
        assert_eq!(
            validate_history_indexes(&changed),
            Err(SnapshotViolation::InvalidHistory),
            "{field}"
        );
    }
    let initial = NormalizedLifecycle::new(identity(), HookProviderKind::Claude, CLAUDE_VERSION);
    validate_history_indexes(&initial).unwrap();
}

#[test]
fn full_replay_rejects_invalid_identity_unreachable_events_and_forged_final_state() {
    let original = reducer();
    let mut changed = original.state().clone();
    changed.expected_identity.invocation_id = uuid::Uuid::nil();
    assert_eq!(
        validate_full_replay(&changed),
        Err(SnapshotViolation::InvalidState)
    );
    let mut changed = original.state().clone();
    changed.history.first_entry().unwrap().get_mut().event = EventKind::UserPromptSubmit;
    assert_eq!(
        validate_full_replay(&changed),
        Err(SnapshotViolation::InvalidHistory)
    );
    let mut changed = original.state().clone();
    changed.turn = TurnState::Idle;
    assert_eq!(
        validate_full_replay(&changed),
        Err(SnapshotViolation::InvalidState)
    );
    validate_reachability(original.state(), original.revision).unwrap();
}

#[test]
fn oversized_history_is_rejected_before_replay_even_when_indexes_are_consistent() {
    let mut state = reducer().state;
    let first = state.history.first_key_value().unwrap().1.clone();
    state.history.clear();
    state.event_ids.clear();
    for offset in 0..=MAX_REPLAY_WINDOW {
        let sequence = EventSequence(crate::FIRST_SEQUENCE + offset as u64);
        let mut event = first.clone();
        event.event_id = uuid::Uuid::new_v4();
        event.sequence = sequence;
        state.event_ids.insert(event.event_id, sequence);
        state.history.insert(sequence, event);
        state.last_sequence = sequence;
    }
    assert_eq!(
        validate_history_indexes(&state),
        Err(SnapshotViolation::InvalidHistory)
    );
}
