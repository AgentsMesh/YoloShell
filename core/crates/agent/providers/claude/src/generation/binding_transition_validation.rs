use super::binding_transition::{PendingBindingTransition, TRANSITION_INVALID};
use super::{ClaudeGenerationAuthority, constants::*, validation};
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use anyhow::{Result, ensure};
use std::path::Path;

pub(super) fn validate_shape(transition: &PendingBindingTransition) -> Result<()> {
    let evidence = &transition.predecessor;
    let target = &transition.target_binding;
    let predecessor = &transition.predecessor_authority;
    ensure!(!transition.invocation_id.is_nil(), TRANSITION_INVALID);
    ensure!(
        transition.publication_generation >= INITIAL_PUBLICATION_GENERATION,
        TRANSITION_INVALID
    );
    ensure!(
        transition.predecessor_publication_generation >= INITIAL_PUBLICATION_GENERATION,
        TRANSITION_INVALID
    );
    ensure!(
        transition.predecessor_revision >= INITIAL_REVISION,
        TRANSITION_INVALID
    );
    ensure!(
        transition.target_revision > transition.predecessor_revision,
        TRANSITION_INVALID
    );
    ensure!(
        predecessor.invocation_id == transition.invocation_id
            && predecessor.revision == transition.predecessor_revision
            && predecessor.state == transition.predecessor_state
            && super::record::validate_loaded(predecessor).is_ok(),
        TRANSITION_INVALID
    );
    ensure!(
        match transition.predecessor_publication {
            Some(_) => predecessor.binding_revision == Some(predecessor.revision),
            None => predecessor.binding_revision.is_none(),
        },
        TRANSITION_INVALID
    );
    ensure!(
        binding_provenance_matches(predecessor, &evidence.binding),
        TRANSITION_INVALID
    );
    ensure!(base_shape(&evidence.binding), TRANSITION_INVALID);
    ensure!(base_shape(target), TRANSITION_INVALID);
    ensure!(target.state == transition.target_state, TRANSITION_INVALID);
    ensure!(
        stable_subject(&evidence.binding, target),
        TRANSITION_INVALID
    );
    ensure!(
        Path::new(&evidence.binding.evidence_path) == evidence.transcript_path,
        TRANSITION_INVALID
    );
    ensure!(
        validation::valid_transcript(
            Path::new(&target.data_home),
            Path::new(&target.evidence_path),
            &target.provider_session_id
        ),
        TRANSITION_INVALID
    );
    ensure!(availability_matches(target), TRANSITION_INVALID);
    ensure!(
        binding_process_shape_for_state(&evidence.binding, transition.predecessor_state),
        TRANSITION_INVALID
    );
    ensure!(binding_process_shape(target), TRANSITION_INVALID);
    ensure!(
        detection_time_matches(&evidence.binding, target),
        TRANSITION_INVALID
    );
    ensure!(
        target.updated_at_ms >= evidence.binding.updated_at_ms,
        TRANSITION_INVALID
    );
    ensure!(transition.proves_target_shape(), TRANSITION_INVALID);
    Ok(())
}

pub(super) fn target_matches(
    transition: &PendingBindingTransition,
    authority: &ClaudeGenerationAuthority,
) -> bool {
    let transcript = Path::new(&transition.target_binding.evidence_path);
    let transcript_matches = match transition.target_state {
        AgentState::Unknown => {
            authority.transcript_path.is_none() && authority.transcript_identity.is_none()
        }
        _ => {
            authority.transcript_path.as_deref() == Some(transcript)
                && authority.transcript_identity_is_current()
        }
    };
    transcript_matches
        && binding_identity_matches(authority, &transition.target_binding, transcript)
        && availability_matches(&transition.target_binding)
}

pub(super) fn binding_identity_matches(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
    transcript: &Path,
) -> bool {
    base_shape(binding)
        && binding.terminal_id == authority.terminal_id
        && binding.provider_session_id == authority.provider_session_id
        && binding.cwd == authority.cwd.to_string_lossy()
        && binding.data_home == authority.data_home.to_string_lossy()
        && Path::new(&binding.evidence_path) == transcript
        && validation::valid_transcript(
            &authority.data_home,
            transcript,
            &authority.provider_session_id,
        )
        && process_matches(binding, authority)
}

fn base_shape(binding: &AgentBinding) -> bool {
    binding.schema_version == crate::agent::constants::BINDING_SCHEMA_VERSION
        && binding.kind == AgentKind::ClaudeCode
        && binding.executable == crate::agent::claude::constants::EXECUTABLE
        && binding.recovery_availability != AgentRecoveryAvailability::UnmanagedLaunch
}

fn stable_subject(left: &AgentBinding, right: &AgentBinding) -> bool {
    left.terminal_id == right.terminal_id
        && left.executable == right.executable
        && left.data_home == right.data_home
}

fn detection_time_matches(previous: &AgentBinding, target: &AgentBinding) -> bool {
    if previous.provider_session_id == target.provider_session_id {
        target.detected_at_ms == previous.detected_at_ms
    } else {
        target.detected_at_ms == target.updated_at_ms
    }
}

fn availability_matches(binding: &AgentBinding) -> bool {
    binding.recovery_availability
        == if binding.state == AgentState::Unknown {
            AgentRecoveryAvailability::Unavailable
        } else {
            AgentRecoveryAvailability::Available
        }
}

fn binding_process_shape(binding: &AgentBinding) -> bool {
    binding_process_shape_for_state(binding, binding.state)
}

fn binding_process_shape_for_state(binding: &AgentBinding, state: AgentState) -> bool {
    match state {
        AgentState::Active => false,
        AgentState::Exited => binding.process_pid.is_none() && binding.process_start_time.is_none(),
        AgentState::Running
        | AgentState::WaitingForInput
        | AgentState::WaitingForPermission
        | AgentState::Unknown => {
            binding.process_pid.is_some() && binding.process_start_time.is_some()
        }
    }
}

fn process_matches(binding: &AgentBinding, authority: &ClaudeGenerationAuthority) -> bool {
    match authority.state {
        AgentState::Active => false,
        AgentState::Exited => binding.process_pid.is_none() && binding.process_start_time.is_none(),
        AgentState::Running
        | AgentState::WaitingForInput
        | AgentState::WaitingForPermission
        | AgentState::Unknown => {
            binding.process_pid == Some(authority.process.pid)
                && binding.process_start_time == Some(authority.process.start_time)
        }
    }
}

fn binding_provenance_matches(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
) -> bool {
    authority.binding_subject_matches(binding)
        && binding.process_pid == Some(authority.process.pid)
        && binding.process_start_time == Some(authority.process.start_time)
        && authority.transcript_path.as_deref() == Some(Path::new(&binding.evidence_path))
        && authority.transcript_identity_is_current()
        && binding.detected_at_ms >= 0
        && binding.updated_at_ms >= binding.detected_at_ms
}
