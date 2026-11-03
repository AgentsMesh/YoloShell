use super::super::ClaudeGenerationAuthority;
use super::super::constants::PROCESS_EXIT_PROOF_UNAVAILABLE;
use super::super::termination::SessionEndBoundary;
use crate::agent::{AgentBinding, AgentRecoveryAvailability, AgentState};
use anyhow::{Result, ensure};

pub(super) fn same_process(
    authority: &ClaudeGenerationAuthority,
    process: crate::process::ProcessIdentity,
) -> bool {
    authority.process.pid == process.pid && authority.process.start_time == process.start_time
}

pub(super) fn process_is_gone(process: crate::process::ProcessIdentity) -> bool {
    matches!(
        crate::process::process_identity_relation(process.pid, process.start_time),
        crate::process::ProcessIdentityRelation::Gone
    )
}

pub(super) fn ensure_process_gone(process: crate::process::ProcessIdentity) -> Result<()> {
    ensure!(process_is_gone(process), PROCESS_EXIT_PROOF_UNAVAILABLE);
    Ok(())
}

pub(super) fn final_candidate(authority: &ClaudeGenerationAuthority) -> bool {
    authority.termination_candidate.is_some_and(|candidate| {
        candidate.valid() && candidate.boundary == SessionEndBoundary::FinalCandidate
    })
}

pub(super) fn unknown_binding_matches(
    authority: &ClaudeGenerationAuthority,
    binding: Option<&AgentBinding>,
) -> bool {
    binding.is_some_and(|binding| {
        binding.state == AgentState::Unknown
            && binding.recovery_availability == AgentRecoveryAvailability::Unavailable
            && authority.matches_unknown_binding(binding)
    })
}
