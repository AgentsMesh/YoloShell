use super::*;
use crate::agent::claude::generation::{AuthorityRuntimeIdentity, ClaudeGenerationAuthority};
use std::path::Path;

pub(super) fn canonical(
    authority: &ClaudeGenerationAuthority,
    current: &AgentBinding,
    detected: &AgentBinding,
    runtime: AuthorityRuntimeIdentity,
) -> Option<AgentBinding> {
    if !admissible(authority, current, detected, runtime) {
        return None;
    }
    if authority.matches_binding(current) {
        return Some(current.clone());
    }
    if current.state == AgentState::Unknown {
        authority.binding(Some(current))
    } else {
        None
    }
}

pub(super) fn retain_rejected_unmanaged(
    authority: &ClaudeGenerationAuthority,
    current: &AgentBinding,
    detected: &AgentBinding,
    runtime: AuthorityRuntimeIdentity,
) -> Option<AgentBinding> {
    (detected.recovery_availability == AgentRecoveryAvailability::UnmanagedLaunch
        && current_admissible(authority, current, runtime))
    .then(|| current.clone())
}

fn admissible(
    authority: &ClaudeGenerationAuthority,
    current: &AgentBinding,
    detected: &AgentBinding,
    runtime: AuthorityRuntimeIdentity,
) -> bool {
    current_admissible(authority, current, runtime)
        && detected.state == AgentState::Unknown
        && detected.recovery_availability == AgentRecoveryAvailability::UnmanagedLaunch
        && super::observation::same(detected, current)
        && detected.detected_at_ms >= 0
        && detected.updated_at_ms >= detected.detected_at_ms
        && native_claude_executable(&detected.executable)
}

fn current_admissible(
    authority: &ClaudeGenerationAuthority,
    current: &AgentBinding,
    runtime: AuthorityRuntimeIdentity,
) -> bool {
    authority.matches_runtime(runtime)
        && authority.binding_revision == Some(authority.revision)
        && !authority.discover_session
        && active(authority.state)
        && authority.transcript_identity_is_current()
        && authority.binding_subject_matches(current)
        && current.process_pid == Some(authority.process.pid)
        && current.process_start_time == Some(authority.process.start_time)
        && current.recovery_availability == AgentRecoveryAvailability::Available
        && (authority.matches_binding(current) || current.state == AgentState::Unknown)
}

fn active(state: AgentState) -> bool {
    matches!(state, AgentState::Running | AgentState::WaitingForInput)
}

fn native_claude_executable(executable: &str) -> bool {
    let name = Path::new(executable)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(executable);
    crate::agent::claude::constants::EXECUTABLE_NAMES.contains(&name)
}
