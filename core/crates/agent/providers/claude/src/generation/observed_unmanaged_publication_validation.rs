use super::ClaudeGenerationAuthority;
use crate::agent::claude::ClaudeAdapter;
use crate::agent::{
    AgentAdapter, AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState, ObservedProcess,
};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(super) fn authority_transcript(
    authority: &ClaudeGenerationAuthority,
) -> Result<(PathBuf, crate::paths::identity::ObjectIdentity)> {
    Ok((
        authority
            .transcript_path
            .clone()
            .context(super::constants::DURABLE_BINDING_INVALID)?,
        authority
            .transcript_identity
            .context(super::constants::DURABLE_BINDING_INVALID)?,
    ))
}

pub(super) fn matches_publishable_authority(
    authority: &ClaudeGenerationAuthority,
    transcript: &Path,
) -> bool {
    !authority.discover_session
        && matches_publishable_state(authority)
        && super::validation::valid_transcript(
            &authority.data_home,
            transcript,
            &authority.provider_session_id,
        )
        && transcript_relation(authority, transcript)
}

fn matches_publishable_state(authority: &ClaudeGenerationAuthority) -> bool {
    matches!(
        authority.state,
        AgentState::Running | AgentState::WaitingForInput
    ) || (authority.state == AgentState::Unknown
        && authority.termination_candidate.is_some()
        && authority.transcript_path.is_none()
        && authority.transcript_identity.is_none())
}

fn transcript_relation(authority: &ClaudeGenerationAuthority, transcript: &Path) -> bool {
    match authority.state {
        AgentState::Running | AgentState::WaitingForInput => {
            authority.transcript_path.as_deref() == Some(transcript)
                && authority.transcript_identity_is_current()
        }
        AgentState::Unknown => authority.transcript_path.is_none(),
        AgentState::Active | AgentState::Exited | AgentState::WaitingForPermission => false,
    }
}

pub(super) fn matches_process(
    authority: &ClaudeGenerationAuthority,
    process: &ObservedProcess,
) -> bool {
    process.identity_matches(authority.process.pid, authority.process.start_time)
        && process.is_foreground()
        && process.cwd.as_deref() == Some(authority.cwd.as_path())
        && ClaudeAdapter.matches_process(process)
}

pub(super) fn matches_binding(
    authority: &ClaudeGenerationAuthority,
    process: &ObservedProcess,
    binding: &AgentBinding,
    transcript: &Path,
) -> bool {
    binding.schema_version == crate::agent::constants::BINDING_SCHEMA_VERSION
        && binding.terminal_id == authority.terminal_id
        && binding.kind == AgentKind::ClaudeCode
        && binding.provider_session_id == authority.provider_session_id
        && binding.executable == process.executable
        && binding.cwd == authority.cwd.to_string_lossy()
        && Path::new(&binding.evidence_path) == transcript
        && binding.data_home == authority.data_home.to_string_lossy()
        && binding.process_pid == Some(authority.process.pid)
        && binding.process_start_time == Some(authority.process.start_time)
        && binding.state == AgentState::Unknown
        && binding.recovery_availability == AgentRecoveryAvailability::UnmanagedLaunch
        && binding.detected_at_ms >= 0
        && binding.updated_at_ms >= binding.detected_at_ms
}
