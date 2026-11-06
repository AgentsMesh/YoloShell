pub(crate) mod provider;
pub(crate) mod scope;

use crate::identity::EventSequence;
use crate::snapshot::SnapshotViolation;
use crate::snapshot::validation::provider::validate_provider_state;
use crate::snapshot::validation::scope::validate_scopes;
use crate::state::{
    AttentionState, ChildLifecycle, InstrumentationState, NormalizedLifecycle, ProcessState,
    TerminationCandidate, TurnState,
};

pub(crate) fn validate_lifecycle(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    state
        .expected_identity
        .validate()
        .map_err(|_| SnapshotViolation::InvalidState)?;
    require(
        state.generation == state.expected_identity.generation
            && state.provider_session_id.as_deref().is_none_or(nonempty)
            && state.provider_session_id.is_some() == (state.last_sequence > EventSequence::ZERO)
            && state.history.is_empty() == (state.last_sequence == EventSequence::ZERO)
            && state.event_ids.is_empty() == (state.last_sequence == EventSequence::ZERO)
            && state.revoked == state.revocation.is_some()
            && state
                .revocation
                .as_ref()
                .is_none_or(|evidence| evidence.is_valid_for(state.expected_identity.process))
            && (state.termination_candidate.is_none() || state.provider_session_id.is_some())
            && state.final_hook_seen
                == matches!(
                    state.termination_candidate,
                    Some(TerminationCandidate::Final)
                ),
    )?;
    crate::lifecycle_metadata::validate(state)?;
    require(crate::state::capacity::is_within_limit(state))?;
    validate_scopes(state)?;
    validate_provider_state(state)?;
    if state.revoked {
        return validate_revoked(state);
    }
    match state.process {
        ProcessState::Alive => validate_alive(state),
        ProcessState::Exited => validate_exited(state),
        ProcessState::Unknown => Err(SnapshotViolation::InvalidState),
    }
}

fn validate_alive(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    match state.instrumentation {
        InstrumentationState::Pending => validate_initial(state),
        InstrumentationState::Authoritative => {
            require(state.provider_session_id.as_deref().is_some_and(nonempty))?;
            if state.termination_candidate.is_some() {
                validate_terminal_root(state)
            } else {
                validate_active_root(state)
            }
        }
        InstrumentationState::Unavailable => validate_degraded(state),
        InstrumentationState::Unmanaged => Err(SnapshotViolation::InvalidState),
    }
}

fn validate_degraded(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    require(
        state.provider_session_id.as_deref().is_some_and(nonempty)
            && state.turn == TurnState::Unknown
            && state.attention == AttentionState::None
            && state.active_tools.is_empty()
            && state.children.is_empty()
            && state.stop_candidate.is_none()
            && state.termination_candidate.is_none()
            && !state.final_hook_seen
            && state.compaction.is_none()
            && state.child_compactions.is_empty(),
    )
}

fn validate_initial(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    require(
        state.provider_session_id.is_none()
            && state.last_sequence == EventSequence::ZERO
            && state.history.is_empty()
            && state.event_ids.is_empty()
            && state.current_cwd.is_none()
            && state.current_transcript_path.is_none()
            && state.turn == TurnState::Unknown
            && state.attention == AttentionState::None
            && state.active_tools.is_empty()
            && state.children.is_empty()
            && state.stop_candidate.is_none()
            && state.termination_candidate.is_none()
            && state.compaction.is_none()
            && state.child_compactions.is_empty(),
    )
}

fn validate_active_root(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    require(
        !matches!(state.turn, TurnState::Unknown)
            && (state.turn == TurnState::Compacting) == state.compaction.is_some()
            && (state.turn != TurnState::StopCandidate || state.stop_candidate.is_some())
            && (state.turn != TurnState::Idle
                || (state.active_tools.is_empty() && state.stop_candidate.is_none())),
    )
}

fn validate_terminal_root(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    require(
        state.turn == TurnState::Unknown
            && state.attention == AttentionState::None
            && state.active_tools.is_empty()
            && state.stop_candidate.is_none()
            && state.compaction.is_none(),
    )
}

fn validate_exited(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    let turn = if state.final_hook_seen {
        require(state.instrumentation == InstrumentationState::Authoritative)?;
        TurnState::Idle
    } else {
        require(state.instrumentation == InstrumentationState::Unavailable)?;
        TurnState::Unknown
    };
    validate_inactive(state, turn)
}

fn validate_revoked(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    require(
        state.process == ProcessState::Unknown
            && state.instrumentation == InstrumentationState::Unavailable
            && state.termination_candidate.is_none()
            && !state.final_hook_seen,
    )?;
    validate_inactive(state, TurnState::Unknown)
}

fn validate_inactive(
    state: &NormalizedLifecycle,
    expected_turn: TurnState,
) -> Result<(), SnapshotViolation> {
    require(
        state.turn == expected_turn
            && state.attention == AttentionState::None
            && state.active_tools.is_empty()
            && state.stop_candidate.is_none()
            && state.compaction.is_none()
            && state.child_compactions.is_empty()
            && state
                .children
                .values()
                .all(|child| inactive_child(child, expected_turn)),
    )
}

fn inactive_child(child: &ChildLifecycle, turn: TurnState) -> bool {
    !child.active
        && child.turn == turn
        && child.attention == AttentionState::None
        && child.active_tools.is_empty()
        && child.stop_candidate.is_none()
}

fn nonempty(value: &str) -> bool {
    !value.is_empty()
}

fn require(condition: bool) -> Result<(), SnapshotViolation> {
    condition
        .then_some(())
        .ok_or(SnapshotViolation::InvalidState)
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/snapshot_validation_matrix_tests.rs"]
mod matrix_tests;
