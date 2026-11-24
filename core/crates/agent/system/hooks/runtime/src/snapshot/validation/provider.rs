use crate::identity::HookProviderKind;
use crate::snapshot::SnapshotViolation;
use crate::state::{AttentionState, NormalizedLifecycle, TerminationCandidate, TurnState};

pub(crate) fn validate_provider_state(
    state: &NormalizedLifecycle,
) -> Result<(), SnapshotViolation> {
    if state.provider != HookProviderKind::Codex {
        return Ok(());
    }
    let invalid_termination = matches!(
        state.termination_candidate,
        Some(TerminationCandidate::Clear | TerminationCandidate::Resume)
    );
    let invalid_root = invalid_turn(state.turn)
        || invalid_attention(state.attention)
        || (state.turn == TurnState::Idle && state.attention != AttentionState::None)
        || (state.attention != AttentionState::None
            && state
                .compaction
                .is_some_and(|phase| phase.prior_turn() == TurnState::Idle))
        || state
            .compaction
            .is_some_and(|phase| invalid_turn(phase.prior_turn()));
    let invalid_child = state
        .children
        .values()
        .any(|child| invalid_turn(child.turn) || invalid_attention(child.attention))
        || state
            .child_compactions
            .values()
            .any(|phase| invalid_turn(phase.prior_turn()));
    if invalid_termination || invalid_root || invalid_child {
        return Err(SnapshotViolation::InvalidState);
    }
    Ok(())
}

fn invalid_turn(turn: TurnState) -> bool {
    turn == TurnState::Failed
}

fn invalid_attention(attention: AttentionState) -> bool {
    matches!(
        attention,
        AttentionState::Permission | AttentionState::UserInput | AttentionState::Elicitation
    )
}
