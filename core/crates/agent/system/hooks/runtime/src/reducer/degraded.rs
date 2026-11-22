use crate::event::{EventKind, HookEnvelope, SessionStartSource};
use crate::state::{
    AttentionState, InstrumentationState, NormalizedLifecycle, ProcessState, TurnState,
};

pub(crate) fn is_active(state: &NormalizedLifecycle) -> bool {
    state.process == ProcessState::Alive
        && state.instrumentation == InstrumentationState::Unavailable
}

pub(crate) fn restores_authority(envelope: &HookEnvelope) -> bool {
    matches!(
        &envelope.event,
        EventKind::SessionStart {
            source: SessionStartSource::Clear
                | SessionStartSource::Resume
                | SessionStartSource::Fork
        }
    )
}

pub(crate) fn canonicalize(state: &mut NormalizedLifecycle) {
    state.instrumentation = InstrumentationState::Unavailable;
    state.turn = TurnState::Unknown;
    state.attention = AttentionState::None;
    state.active_tools.clear();
    state.children.clear();
    state.stop_candidate = None;
    state.termination_candidate = None;
    state.final_hook_seen = false;
    state.compaction = None;
    state.child_compactions.clear();
}
