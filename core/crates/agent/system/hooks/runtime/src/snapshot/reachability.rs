use crate::admission::validate_envelope;
use crate::constants::MAX_REPLAY_WINDOW;
use crate::identity::EventSequence;
use crate::reducer::Reducer;
use crate::snapshot::SnapshotViolation;
use crate::snapshot::revision::AuthorityRevision;
use crate::state::{NormalizedLifecycle, ProcessState};

pub(crate) fn validate_reachability(
    state: &NormalizedLifecycle,
    revision: AuthorityRevision,
) -> Result<(), SnapshotViolation> {
    validate_history_indexes(state)?;
    if let Some(evidence) = state.revocation.as_ref() {
        return crate::snapshot::revocation::validation::validate(state, revision, evidence);
    }
    let starts_at_first = state.history.keys().next().copied() == Some(EventSequence::FIRST);
    if starts_at_first || state.last_sequence == EventSequence::ZERO {
        validate_full_replay(state)?;
    }
    Ok(())
}

fn validate_history_indexes(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    if state.history.len() > MAX_REPLAY_WINDOW
        || state.event_ids.len() != state.history.len()
        || state
            .history
            .values()
            .any(|envelope| validate_envelope(state, envelope).is_err())
    {
        return Err(SnapshotViolation::InvalidHistory);
    }
    if state.history.is_empty() {
        return (state.last_sequence == EventSequence::ZERO)
            .then_some(())
            .ok_or(SnapshotViolation::InvalidHistory);
    }
    let mut previous = None;
    for (sequence, envelope) in &state.history {
        if envelope.sequence != *sequence
            || previous.is_some_and(|prior: EventSequence| prior.next() != Some(*sequence))
            || state.event_ids.get(&envelope.event_id) != Some(sequence)
        {
            return Err(SnapshotViolation::InvalidHistory);
        }
        previous = Some(*sequence);
    }
    if previous != Some(state.last_sequence) {
        return Err(SnapshotViolation::InvalidHistory);
    }
    Ok(())
}

fn validate_full_replay(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    let mut replay = Reducer::new(
        state.expected_identity.clone(),
        state.provider.clone(),
        state.provider_version,
    )
    .map_err(|_| SnapshotViolation::InvalidState)?;
    for envelope in state.history.values() {
        replay
            .apply(envelope.clone())
            .map_err(|_| SnapshotViolation::InvalidHistory)?;
    }
    if state.process == ProcessState::Exited {
        replay.process_exit();
    }
    if replay.state != *state {
        return Err(SnapshotViolation::InvalidState);
    }
    Ok(())
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/snapshot_history_matrix_tests.rs"]
mod matrix_tests;
