use crate::constants::MAX_REPLAY_WINDOW;
use crate::event::HookEnvelope;
use crate::state::NormalizedLifecycle;

pub(crate) fn record(state: &mut NormalizedLifecycle, envelope: HookEnvelope) {
    state.last_sequence = envelope.sequence;
    state.event_ids.insert(envelope.event_id, envelope.sequence);
    state.history.insert(envelope.sequence, envelope);
    trim(state);
}

fn trim(state: &mut NormalizedLifecycle) {
    while state.history.len() > MAX_REPLAY_WINDOW {
        let Some(sequence) = state.history.keys().next().copied() else {
            break;
        };
        state.history.remove(&sequence);
    }
    while state.event_ids.len() > MAX_REPLAY_WINDOW {
        let Some(event_id) = state
            .event_ids
            .iter()
            .min_by_key(|(_, sequence)| *sequence)
            .map(|(id, _)| *id)
        else {
            break;
        };
        state.event_ids.remove(&event_id);
    }
    let history = &state.history;
    state.children.retain(|agent, child| {
        child.active
            || history
                .values()
                .any(|event| event.agent_id.as_deref() == Some(agent.as_str()))
    });
    state
        .child_compactions
        .retain(|agent, _| state.children.contains_key(agent));
}
