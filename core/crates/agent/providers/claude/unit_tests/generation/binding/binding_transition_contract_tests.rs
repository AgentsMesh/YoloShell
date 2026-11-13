use super::binding_transition_contract::*;
use crate::agent::AgentState;
use crate::agent::claude::generation::{
    SessionStartSource, binding_transition::BindingTransitionProof, validation,
};

#[test]
fn persisted_predecessor_cannot_refer_to_a_different_process_or_transcript_source() {
    for change_pid in [false, true] {
        let case = PendingCase::state(AgentState::WaitingForInput);
        let mut changed = case.set.clone();
        let prior = &mut changed
            .pending_binding_transition
            .as_mut()
            .unwrap()
            .predecessor
            .binding;
        if change_pid {
            prior.process_pid = prior.process_pid.map(|pid| pid + PROCESS_STEP);
        } else {
            prior.process_start_time = prior.process_start_time.map(|time| time + IDENTITY_STEP);
        }
        case.rejected(changed);
    }
    let case = PendingCase::state(AgentState::WaitingForInput);
    let mut changed = case.set.clone();
    let transition = changed.pending_binding_transition.as_mut().unwrap();
    transition.predecessor.transcript_path = transition
        .predecessor
        .transcript_path
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(OTHER_PROJECT)
        .join(transition.predecessor.transcript_path.file_name().unwrap());
    assert!(validation::valid_transcript(
        &transition.predecessor_authority.data_home,
        &transition.predecessor.transcript_path,
        &transition.predecessor_authority.provider_session_id
    ));
    case.rejected(changed);
}

#[test]
fn live_and_exited_publications_reject_partial_process_identities() {
    for state in [AgentState::WaitingForInput, AgentState::Exited] {
        for pid_field in [false, true] {
            let case = PendingCase::state(state);
            assert!(state_proof(case.transition()));
            let mut changed = case.set.clone();
            let binding = &mut changed
                .pending_binding_transition
                .as_mut()
                .unwrap()
                .target_binding;
            if state == AgentState::Exited {
                if pid_field {
                    binding.process_pid = Some(case.authority.process.pid);
                } else {
                    binding.process_start_time = Some(case.authority.process.start_time);
                }
            } else if pid_field {
                binding.process_pid = None;
            } else {
                binding.process_start_time = None;
            }
            case.rejected(changed);
        }
        let case = PendingCase::state(state);
        case.commit(&case.transition().target_binding);
    }
}

#[test]
fn same_session_preserves_detection_time_and_accepts_equal_publication_timestamp() {
    let mut case = PendingCase::state(AgentState::WaitingForInput);
    case.set
        .pending_binding_transition
        .as_mut()
        .unwrap()
        .target_binding
        .updated_at_ms = PREVIOUS_UPDATE;
    let mut retimed = case.set.clone();
    retimed
        .pending_binding_transition
        .as_mut()
        .unwrap()
        .target_binding
        .detected_at_ms += TIME_STEP;
    case.rejected(retimed);
    let mut backwards = case.set.clone();
    backwards
        .pending_binding_transition
        .as_mut()
        .unwrap()
        .target_binding
        .updated_at_ms = PREVIOUS_UPDATE - TIME_STEP;
    assert!(
        backwards
            .pending_binding_transition
            .as_ref()
            .unwrap()
            .target_binding
            .updated_at_ms
            >= DETECTED
    );
    case.rejected(backwards);
    case.commit(&case.transition().target_binding);
}

#[test]
fn restarted_agent_detection_begins_at_its_own_publication_without_rewriting_prior_evidence() {
    let case = PendingCase::restarted();
    let transition = case.transition();
    assert_ne!(
        transition.predecessor.binding.provider_session_id,
        transition.target_binding.provider_session_id
    );
    assert_eq!(
        transition.target_binding.detected_at_ms,
        transition.target_binding.updated_at_ms
    );
    let mut inherited_time = case.set.clone();
    inherited_time
        .pending_binding_transition
        .as_mut()
        .unwrap()
        .target_binding
        .detected_at_ms = DETECTED;
    case.rejected(inherited_time);
    case.commit(&case.transition().target_binding);
}

#[test]
fn valid_event_shape_cannot_replace_the_staged_restart_source_or_event_sequence() {
    for source_changed in [false, true] {
        let case = PendingCase::restarted();
        let mut changed = case.set.clone();
        let transition = changed.pending_binding_transition.as_mut().unwrap();
        let BindingTransitionProof::Recovery { events } = &mut transition.proof else {
            unreachable!()
        };
        let [
            crate::agent::claude::generation::GenerationRecoveryEvent {
                sequence, event, ..
            },
        ] = events.as_mut_slice()
        else {
            unreachable!()
        };
        if source_changed {
            let crate::agent::claude::generation::GenerationEvent::SessionStart { source, .. } =
                event
            else {
                unreachable!()
            };
            *source = SessionStartSource::Fresh;
        } else {
            *sequence = transition.predecessor_authority.last_event_sequence;
        }
        validation::validate_event(event, &transition.predecessor_authority.data_home).unwrap();
        case.rejected(changed);
    }
}
