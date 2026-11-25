use super::binding_transition::BindingTransitionProof;
use super::{ClaudeGenerationAuthority, GenerationApplyDisposition};
use crate::agent::AgentState;

pub(super) fn candidate(
    predecessor: &ClaudeGenerationAuthority,
    proof: &BindingTransitionProof,
) -> Option<ClaudeGenerationAuthority> {
    match proof {
        BindingTransitionProof::Event {
            sequence,
            event,
            transcript_identity,
        } => super::transition::apply(predecessor, *sequence, event, *transcript_identity)
            .ok()
            .and_then(|(candidate, disposition)| {
                (disposition == GenerationApplyDisposition::Applied).then_some(candidate)
            }),
        BindingTransitionProof::Recovery { events } => {
            super::recovery_store::fold_recovery(predecessor, events)
                .ok()
                .and_then(|(candidate, applied)| applied.then_some(candidate))
        }
        BindingTransitionProof::StateMutation { target_state } => {
            state_candidate(predecessor, *target_state)
        }
    }
}

fn state_candidate(
    predecessor: &ClaudeGenerationAuthority,
    target_state: AgentState,
) -> Option<ClaudeGenerationAuthority> {
    let mut candidate =
        super::transition_mutation::advance_state(predecessor, target_state).ok()?;
    if target_state == AgentState::Unknown {
        candidate.transcript_path = None;
        candidate.transcript_identity = None;
    }
    Some(candidate)
}
