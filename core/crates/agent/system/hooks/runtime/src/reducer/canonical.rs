use crate::state::{
    AttentionState, InstrumentationState, NormalizedLifecycle, ProcessState, TurnState,
};

pub(crate) fn canonicalize_revoked(
    state: &mut NormalizedLifecycle,
    evidence: crate::revocation::RevocationEvidence,
) {
    state.revoked = true;
    state.revocation = Some(evidence);
    state.process = ProcessState::Unknown;
    state.instrumentation = InstrumentationState::Unavailable;
    state.termination_candidate = None;
    state.final_hook_seen = false;
    normalize_inactive_state(state, TurnState::Unknown);
}

pub(crate) fn normalize_inactive_state(state: &mut NormalizedLifecycle, turn: TurnState) {
    state.turn = turn;
    state.attention = AttentionState::None;
    state.active_tools.clear();
    state.stop_candidate = None;
    state.compaction = None;
    state.child_compactions.clear();
    for child in state.children.values_mut() {
        child.active = false;
        child.turn = turn;
        child.attention = AttentionState::None;
        child.active_tools.clear();
        child.stop_candidate = None;
    }
}
