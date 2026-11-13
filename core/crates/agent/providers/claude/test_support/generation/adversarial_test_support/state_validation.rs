use super::super::ClaudeGenerationAuthority;
use crate::agent::{AgentBinding, AgentRecoveryAvailability, AgentState};
use anyhow::{Result, ensure};
use std::path::Path;

pub(super) const BINDING_IDENTITY_MISMATCH: &str = "Claude durable state binding identity mismatch";

pub(super) fn validate_binding_identity(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
) -> Result<()> {
    if authority.state == AgentState::Unknown {
        ensure!(
            binding.schema_version == crate::agent::constants::BINDING_SCHEMA_VERSION
                && binding.kind == crate::agent::AgentKind::ClaudeCode
                && binding.terminal_id == authority.terminal_id
                && binding.provider_session_id == authority.provider_session_id
                && binding.executable == crate::agent::claude::constants::EXECUTABLE
                && binding.cwd == authority.cwd.to_string_lossy()
                && binding.data_home == authority.data_home.to_string_lossy()
                && binding.state == AgentState::Unknown
                && binding.recovery_availability == AgentRecoveryAvailability::Unavailable,
            BINDING_IDENTITY_MISMATCH
        );
        return Ok(());
    }
    let evidence = Path::new(&binding.evidence_path);
    let evidence_matches = authority.transcript_path.as_deref() == Some(evidence)
        && authority.transcript_identity.is_some_and(|identity| {
            super::super::super::authoritative_evidence::named_transcript_identity(
                &authority.data_home,
                evidence,
            )
            .ok()
            .flatten()
                == Some(identity)
        });
    let process_matches = strict_process_matches(authority, binding);
    ensure!(
        binding.schema_version == crate::agent::constants::BINDING_SCHEMA_VERSION
            && binding.terminal_id == authority.terminal_id
            && binding.provider_session_id == authority.provider_session_id
            && binding.kind == crate::agent::AgentKind::ClaudeCode
            && binding.executable == crate::agent::claude::constants::EXECUTABLE
            && binding.cwd == authority.cwd.to_string_lossy()
            && binding.data_home == authority.data_home.to_string_lossy()
            && evidence_matches
            && process_matches
            && binding.recovery_availability != AgentRecoveryAvailability::UnmanagedLaunch,
        BINDING_IDENTITY_MISMATCH
    );
    Ok(())
}

fn strict_process_matches(authority: &ClaudeGenerationAuthority, binding: &AgentBinding) -> bool {
    match authority.state {
        AgentState::Running | AgentState::WaitingForInput => {
            binding.process_pid == Some(authority.process.pid)
                && binding.process_start_time == Some(authority.process.start_time)
        }
        AgentState::Exited => binding.process_pid.is_none() && binding.process_start_time.is_none(),
        AgentState::Active | AgentState::Unknown | AgentState::WaitingForPermission => false,
    }
}
