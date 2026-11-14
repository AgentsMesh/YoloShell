use super::binding_transition_contract::*;
use crate::agent::{AgentRecoveryAvailability, AgentState};

#[test]
fn target_binding_cannot_change_session_or_data_home_or_cwd_while_authority_stays_current() {
    for change in [
        (|binding: &mut crate::agent::AgentBinding| binding.terminal_id = uuid::Uuid::new_v4())
            as fn(&mut crate::agent::AgentBinding),
        |binding| binding.data_home.push_str(OTHER_PROJECT),
        |binding| binding.cwd.push_str(OTHER_PROJECT),
    ] {
        let case = PendingCase::state(AgentState::WaitingForInput);
        let mut changed = case.set.clone();
        change(
            &mut changed
                .pending_binding_transition
                .as_mut()
                .unwrap()
                .target_binding,
        );
        case.rejected(changed);
    }
}

#[test]
fn provenance_requires_nonnegative_ordered_detection_time_even_for_valid_authorities() {
    for negative in [false, true] {
        let case = PendingCase::state(AgentState::WaitingForInput);
        let mut changed = case.set.clone();
        let binding = &mut changed
            .pending_binding_transition
            .as_mut()
            .unwrap()
            .predecessor
            .binding;
        if negative {
            binding.detected_at_ms = -TIME_STEP;
        } else {
            binding.updated_at_ms = DETECTED - TIME_STEP;
        }
        case.rejected(changed);
    }
}

#[test]
fn unknown_transition_keeps_process_identity_but_cannot_advertise_available_recovery() {
    let case = PendingCase::state(AgentState::Unknown);
    assert!(case.authority.transcript_path.is_none());
    assert!(case.authority.transcript_identity.is_none());
    let target = &case.transition().target_binding;
    assert_eq!(target.process_pid, Some(case.authority.process.pid));
    assert_eq!(
        target.process_start_time,
        Some(case.authority.process.start_time)
    );
    let mut advertised = case.set.clone();
    advertised
        .pending_binding_transition
        .as_mut()
        .unwrap()
        .target_binding
        .recovery_availability = AgentRecoveryAvailability::Available;
    case.rejected(advertised);
    case.commit(target);
}

#[test]
fn byte_identical_transcript_replacement_cannot_supply_the_predecessor_evidence() {
    let case = PendingCase::state(AgentState::WaitingForInput);
    let transcript = &case.transition().predecessor.transcript_path;
    let bytes = std::fs::read(transcript).unwrap();
    std::fs::rename(transcript, transcript.with_extension(DISPLACED_TRANSCRIPT)).unwrap();
    std::fs::write(transcript, bytes).unwrap();
    case.rejected(case.set.clone());
}

#[test]
fn complete_but_foreign_target_process_identity_cannot_match_the_staged_authority() {
    for change_pid in [false, true] {
        let case = PendingCase::state(AgentState::WaitingForInput);
        let mut changed = case.set.clone();
        let transition = changed.pending_binding_transition.as_mut().unwrap();
        if change_pid {
            transition.target_binding.process_pid = Some(case.authority.process.pid + PROCESS_STEP);
        } else {
            transition.target_binding.process_start_time =
                Some(case.authority.process.start_time + IDENTITY_STEP);
        }
        // This still passes the full transition shape check; the public store
        // must reject the later authority-to-target process identity comparison.
        transition.validate_shape().unwrap();
        case.rejected(changed);
    }
}

#[test]
fn internally_well_formed_publication_numbers_cannot_skip_the_exact_predecessor_generation() {
    let case = PendingCase::state(AgentState::WaitingForInput);
    let mut changed = case.set.clone();
    changed.publication_generation +=
        crate::agent::claude::generation::constants::REVISION_INCREMENT;
    let transition = changed.pending_binding_transition.as_mut().unwrap();
    transition.publication_generation = changed.publication_generation;
    transition.validate_shape().unwrap();
    assert_eq!(
        transition
            .predecessor_publication
            .unwrap()
            .publication_generation,
        transition.predecessor_publication_generation
    );
    case.rejected(changed);
}
