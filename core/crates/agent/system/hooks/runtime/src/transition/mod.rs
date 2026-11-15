pub(crate) mod admission;
mod common;
pub(crate) mod helpers;

use self::common::apply_common;
use crate::event::{EventKind, HookEnvelope, SessionEndReason, SessionStartSource};
use crate::state::{
    AttentionState, ChildLifecycle, InstrumentationState, NormalizedLifecycle, ProcessState,
    TurnState,
};
use crate::transition::helpers::termination_candidate;

pub(crate) fn apply_event(state: &mut NormalizedLifecycle, envelope: &HookEnvelope) {
    match &envelope.event {
        EventKind::SessionStart { source } => {
            let continuation = matches!(source, SessionStartSource::Compact);
            state.provider_session_id = Some(envelope.provider_session_id.clone());
            state.process = ProcessState::Alive;
            state.instrumentation = InstrumentationState::Authoritative;
            state.termination_candidate = None;
            state.final_hook_seen = false;
            if !continuation {
                reset_boundary_state(state);
            }
            if let Some(compaction) = state.compaction.take().filter(|_| continuation) {
                state.turn = compaction.prior_turn();
            } else if !continuation {
                state.turn = TurnState::Idle;
            }
        }
        EventKind::SubagentStart => start_child(state, envelope),
        EventKind::SubagentStop => stop_child(state, envelope),
        _ if envelope.agent_id.is_some() => apply_child_event(state, envelope),
        _ => apply_root_event(state, envelope),
    }
}

fn reset_boundary_state(state: &mut NormalizedLifecycle) {
    state.attention = AttentionState::None;
    state.active_tools.clear();
    state.stop_candidate = None;
    state.compaction = None;
    state.child_compactions.clear();
    state.children.clear();
}

fn start_child(state: &mut NormalizedLifecycle, envelope: &HookEnvelope) {
    if let Some(agent) = envelope.agent_id.clone() {
        state.children.insert(
            agent,
            ChildLifecycle {
                active: true,
                turn: TurnState::Running,
                attention: AttentionState::None,
                active_tools: Default::default(),
                stop_candidate: None,
            },
        );
    }
}

fn stop_child(state: &mut NormalizedLifecycle, envelope: &HookEnvelope) {
    if let Some(agent) = envelope.agent_id.as_deref() {
        state.child_compactions.remove(agent);
        if let Some(child) = state.children.get_mut(agent) {
            child.active = false;
            child.turn = TurnState::Idle;
            child.attention = AttentionState::None;
            child.active_tools.clear();
            child.stop_candidate = None;
        }
    }
}

fn apply_root_event(state: &mut NormalizedLifecycle, envelope: &HookEnvelope) {
    apply_common(
        &envelope.event,
        envelope,
        &mut state.turn,
        &mut state.attention,
        &mut state.active_tools,
        &mut state.stop_candidate,
        &mut state.compaction,
    );
    if let EventKind::SessionEnd { reason } = &envelope.event {
        state.termination_candidate = Some(termination_candidate(reason));
        state.final_hook_seen = matches!(reason, SessionEndReason::Other);
    }
}

fn apply_child_event(state: &mut NormalizedLifecycle, envelope: &HookEnvelope) {
    let Some(agent) = envelope.agent_id.as_deref() else {
        return;
    };
    let mut compaction = state.child_compactions.remove(agent);
    let Some(child) = state.children.get_mut(agent) else {
        return;
    };
    apply_common(
        &envelope.event,
        envelope,
        &mut child.turn,
        &mut child.attention,
        &mut child.active_tools,
        &mut child.stop_candidate,
        &mut compaction,
    );
    if let Some(compaction) = compaction {
        state.child_compactions.insert(agent.to_owned(), compaction);
    }
}
