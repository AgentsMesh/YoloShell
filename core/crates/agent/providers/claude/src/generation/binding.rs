use super::ClaudeGenerationAuthority;
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use std::path::PathBuf;

pub(super) fn from_authority(
    authority: &ClaudeGenerationAuthority,
    previous: Option<&AgentBinding>,
) -> Option<AgentBinding> {
    if authority.state == AgentState::Unknown {
        return None;
    }
    let transcript = authority
        .transcript_path
        .clone()
        .or_else(|| exited_evidence(authority, previous))?;
    let now = crate::model::now_ms();
    let detected_at_ms = previous
        .filter(|binding| {
            binding.kind == AgentKind::ClaudeCode
                && binding.provider_session_id == authority.provider_session_id
        })
        .map_or(now, |binding| binding.detected_at_ms);
    let process = (authority.state != AgentState::Exited)
        .then_some((authority.process.pid, authority.process.start_time));
    Some(AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: authority.terminal_id,
        kind: AgentKind::ClaudeCode,
        provider_session_id: authority.provider_session_id.clone(),
        executable: crate::agent::claude::constants::EXECUTABLE.to_owned(),
        cwd: authority.cwd.to_string_lossy().into_owned(),
        evidence_path: transcript.to_string_lossy().into_owned(),
        data_home: authority.data_home.to_string_lossy().into_owned(),
        process_pid: process.map(|identity| identity.0),
        process_start_time: process.map(|identity| identity.1),
        state: authority.state,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms,
        updated_at_ms: now,
    })
}

fn exited_evidence(
    authority: &ClaudeGenerationAuthority,
    previous: Option<&AgentBinding>,
) -> Option<PathBuf> {
    if authority.state != AgentState::Exited {
        return None;
    }
    let binding = previous?;
    let prior_shape = (binding.state == AgentState::Unknown
        && binding.process_pid == Some(authority.process.pid)
        && binding.process_start_time == Some(authority.process.start_time))
        || (binding.state == AgentState::Exited
            && binding.process_pid.is_none()
            && binding.process_start_time.is_none());
    (prior_shape && subject_matches(authority, binding))
        .then(|| PathBuf::from(&binding.evidence_path))
}

pub(super) fn transition_target(
    authority: &ClaudeGenerationAuthority,
    previous: Option<&AgentBinding>,
) -> Option<AgentBinding> {
    if authority.state != AgentState::Unknown {
        return from_authority(authority, previous);
    }
    let mut binding = previous?.clone();
    if !unknown_subject_matches(authority, &binding) {
        return None;
    }
    binding.state = AgentState::Unknown;
    binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
    binding.updated_at_ms = crate::model::now_ms()
        .max(binding.detected_at_ms)
        .max(binding.updated_at_ms);
    Some(binding)
}

pub(super) fn unknown_matches(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
) -> bool {
    unknown_subject_matches(authority, binding)
        && binding.state == AgentState::Unknown
        && binding.recovery_availability == AgentRecoveryAvailability::Unavailable
}

pub(super) fn subject_matches(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
) -> bool {
    let evidence = std::path::Path::new(&binding.evidence_path);
    binding.schema_version == crate::agent::constants::BINDING_SCHEMA_VERSION
        && binding.kind == AgentKind::ClaudeCode
        && binding.terminal_id == authority.terminal_id
        && binding.provider_session_id == authority.provider_session_id
        && binding.executable == crate::agent::claude::constants::EXECUTABLE
        && binding.cwd == authority.cwd.to_string_lossy()
        && binding.data_home == authority.data_home.to_string_lossy()
        && super::validation::valid_transcript(
            &authority.data_home,
            evidence,
            &authority.provider_session_id,
        )
}

fn unknown_subject_matches(authority: &ClaudeGenerationAuthority, binding: &AgentBinding) -> bool {
    subject_matches(authority, binding)
        && binding.process_pid == Some(authority.process.pid)
        && binding.process_start_time == Some(authority.process.start_time)
        && binding.updated_at_ms >= binding.detected_at_ms
}
