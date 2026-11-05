use super::{ClaudeGenerationAuthority, InvocationMode, SessionStartSource};
use crate::agent::claude::ClaudeAdapter;
use crate::agent::{
    AgentAdapter, AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState, ObservedProcess,
};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

const FIRST_SESSION_EVENT_COUNT: usize = 1;

#[derive(Clone)]
pub(super) struct ManagedShimExecPublication {
    predecessor: AgentBinding,
    shim: PathBuf,
}

impl ManagedShimExecPublication {
    pub(super) fn capture(
        authority: &ClaudeGenerationAuthority,
        successor: &ObservedProcess,
        predecessor: Option<&AgentBinding>,
        shim: &Path,
    ) -> Option<Self> {
        let predecessor = predecessor?;
        matches_transition(authority, successor, predecessor, shim).then(|| Self {
            predecessor: predecessor.clone(),
            shim: shim.to_path_buf(),
        })
    }

    pub(super) fn accepts(&self, existing: Option<&AgentBinding>) -> bool {
        existing == Some(&self.predecessor)
    }

    pub(super) fn verify(
        &self,
        authority: &ClaudeGenerationAuthority,
        successor: &ObservedProcess,
    ) -> Result<()> {
        let now = crate::process::monotonic_time_ns()?;
        ensure!(
            accepts_at(
                authority,
                successor,
                &self.predecessor,
                &self.shim,
                &crate::process::boot_id(),
                now,
            ),
            super::constants::DURABLE_BINDING_INVALID
        );
        Ok(())
    }
}

fn accepts_at(
    authority: &ClaudeGenerationAuthority,
    successor: &ObservedProcess,
    predecessor: &AgentBinding,
    shim: &Path,
    boot_id: &str,
    now: u64,
) -> bool {
    matches_transition(authority, successor, predecessor, shim)
        && crate::process::boot_identity_relation(&authority.handshake_boot_id, boot_id)
            == crate::process::BootIdentityRelation::Same
        && now < authority.handshake_deadline_ns
}

fn matches_transition(
    authority: &ClaudeGenerationAuthority,
    successor: &ObservedProcess,
    predecessor: &AgentBinding,
    shim: &Path,
) -> bool {
    first_session_start(authority)
        && predecessor_matches(authority, predecessor, shim)
        && successor_matches(authority, successor, shim)
}

fn first_session_start(authority: &ClaudeGenerationAuthority) -> bool {
    let Some(applied) = authority.replay_events.front() else {
        return false;
    };
    authority.binding_revision.is_none()
        && !authority.discover_session
        && authority.revision
            == super::constants::INITIAL_REVISION + super::constants::REVISION_INCREMENT
        && authority.generation == super::constants::INITIAL_GENERATION
        && matches!(
            authority.state,
            AgentState::Running | AgentState::WaitingForInput
        )
        && authority.last_event_sequence == crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE
        && authority.replay_events.len() == FIRST_SESSION_EVENT_COUNT
        && applied.sequence == crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE
        && applied.generation == authority.generation
        && matches!(
            &applied.event,
            super::GenerationEvent::SessionStart {
                provider_session_id,
                source,
                cwd,
                transcript_path,
                state,
                ..
            } if provider_session_id == &authority.provider_session_id
                && source_matches(authority.mode, *source)
                && cwd == &authority.cwd
                && Some(transcript_path) == authority.transcript_path.as_ref()
                && state == &authority.state
        )
}

fn source_matches(mode: InvocationMode, source: SessionStartSource) -> bool {
    matches!(
        (mode, source),
        (InvocationMode::Fresh, SessionStartSource::Fresh)
            | (
                InvocationMode::Resume,
                SessionStartSource::Resume | SessionStartSource::Fork
            )
    )
}

fn predecessor_matches(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
    shim: &Path,
) -> bool {
    let Some(transcript) = authority.transcript_path.as_ref() else {
        return false;
    };
    shim.is_absolute()
        && Path::new(&binding.executable) == shim
        && binding.schema_version == crate::agent::constants::BINDING_SCHEMA_VERSION
        && binding.terminal_id == authority.terminal_id
        && binding.kind == AgentKind::ClaudeCode
        && binding.provider_session_id == authority.provider_session_id
        && binding.cwd == authority.cwd.to_string_lossy()
        && Path::new(&binding.evidence_path) == transcript
        && binding.data_home == authority.data_home.to_string_lossy()
        && binding.process_pid == Some(authority.process.pid)
        && binding.process_start_time == Some(authority.process.start_time)
        && binding.state == AgentState::Unknown
        && binding.recovery_availability == AgentRecoveryAvailability::Unavailable
        && binding.detected_at_ms >= 0
        && binding.updated_at_ms >= binding.detected_at_ms
}

fn successor_matches(
    authority: &ClaudeGenerationAuthority,
    process: &ObservedProcess,
    shim: &Path,
) -> bool {
    let executable = Path::new(&process.executable);
    executable.is_absolute()
        && executable != shim
        && process.identity_matches(authority.process.pid, authority.process.start_time)
        && process.is_foreground()
        && process.cwd.as_deref() == Some(authority.cwd.as_path())
        && ClaudeAdapter.matches_process(process)
}

#[cfg(test)]
pub(super) fn accepts_for_test(
    authority: &ClaudeGenerationAuthority,
    successor: &ObservedProcess,
    predecessor: &AgentBinding,
    shim: &Path,
    boot_id: &str,
    now: u64,
) -> bool {
    accepts_at(authority, successor, predecessor, shim, boot_id, now)
}
