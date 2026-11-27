use crate::event::EventKind;
use crate::identity::EventSequence;
use crate::snapshot::SnapshotViolation;
use crate::state::{
    AttentionState, CompactionPhase, NormalizedLifecycle, ProcessState, StopCandidate, TurnState,
};
use std::collections::BTreeSet;

pub(crate) fn validate_scopes(state: &NormalizedLifecycle) -> Result<(), SnapshotViolation> {
    require(
        state.active_tools.iter().all(|tool| nonempty(tool))
            && valid_stop(state.stop_candidate.as_ref())
            && valid_phase(state.compaction, state.last_sequence, true),
    )?;
    let mut phase_sequences = BTreeSet::new();
    if let Some(phase) = state.compaction {
        phase_sequences.insert(phase.started_at());
    }
    validate_anchors(state, None, state.stop_candidate.as_ref(), state.compaction)?;
    for (agent, child) in &state.children {
        let phase = state.child_compactions.get(agent).copied();
        require(
            nonempty(agent)
                && child.active_tools.iter().all(|tool| nonempty(tool))
                && valid_stop(child.stop_candidate.as_ref())
                && valid_phase(phase, state.last_sequence, false),
        )?;
        if let Some(phase) = phase {
            require(phase_sequences.insert(phase.started_at()))?;
        }
        validate_anchors(state, Some(agent), child.stop_candidate.as_ref(), phase)?;
        if child.active {
            require(
                !matches!(child.turn, TurnState::Unknown | TurnState::Idle)
                    && (child.turn == TurnState::Compacting) == phase.is_some()
                    && (child.turn != TurnState::StopCandidate || child.stop_candidate.is_some()),
            )?;
        } else {
            require(
                phase.is_none()
                    && child.turn == inactive_turn(state)
                    && child.attention == AttentionState::None
                    && child.active_tools.is_empty()
                    && child.stop_candidate.is_none(),
            )?;
        }
    }
    require(
        state
            .child_compactions
            .keys()
            .all(|agent| state.children.contains_key(agent)),
    )
}

fn inactive_turn(state: &NormalizedLifecycle) -> TurnState {
    if state.revoked || state.process == ProcessState::Unknown {
        TurnState::Unknown
    } else if state.process == ProcessState::Exited {
        state.turn
    } else {
        TurnState::Idle
    }
}

fn valid_stop(stop: Option<&StopCandidate>) -> bool {
    stop.is_none_or(|candidate| {
        !candidate.event_id.is_nil() && candidate.turn_id.as_deref().is_none_or(nonempty)
    })
}

fn valid_phase(phase: Option<CompactionPhase>, last: EventSequence, idle_allowed: bool) -> bool {
    phase.is_none_or(|phase| {
        phase.started_at() > EventSequence::ZERO
            && phase.started_at() <= last
            && !matches!(
                phase.prior_turn(),
                TurnState::Unknown | TurnState::Compacting
            )
            && (idle_allowed || phase.prior_turn() != TurnState::Idle)
    })
}

fn validate_anchors(
    state: &NormalizedLifecycle,
    agent: Option<&str>,
    stop: Option<&StopCandidate>,
    phase: Option<CompactionPhase>,
) -> Result<(), SnapshotViolation> {
    if let Some(phase) = phase {
        require(phase.prior_turn() != TurnState::StopCandidate || stop.is_some())?;
        if agent.is_none() && phase.prior_turn() == TurnState::Idle {
            require(state.active_tools.is_empty() && stop.is_none())?;
        }
        if let Some(envelope) = state.history.get(&phase.started_at()) {
            require(
                envelope.agent_id.as_deref() == agent
                    && matches!(&envelope.event, EventKind::PreCompact),
            )?;
        }
    }
    if let Some(stop) = stop
        && let Some(sequence) = state.event_ids.get(&stop.event_id)
    {
        let envelope = state
            .history
            .get(sequence)
            .ok_or(SnapshotViolation::InvalidState)?;
        require(
            envelope.agent_id.as_deref() == agent
                && matches!(&envelope.event, EventKind::Stop)
                && envelope.turn_id == stop.turn_id
                && envelope.stop_hook_active.unwrap_or(false) == stop.continued,
        )?;
    }
    Ok(())
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
#[path = "../../../unit_tests/snapshot_scope_matrix_tests.rs"]
mod matrix_tests;
