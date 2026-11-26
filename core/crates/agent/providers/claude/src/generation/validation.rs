use super::SessionEndTermination;
use super::constants::*;
use super::types::{
    AuthorityProcessIdentity, ClaudeGenerationAuthority, GenerationBegin, GenerationEvent,
};
use crate::agent::AgentState;
use anyhow::{Result, ensure};
use std::path::{Component, Path};
use uuid::Uuid;

pub(super) fn validate_begin(begin: &GenerationBegin) -> Result<()> {
    ensure!(
        !begin.terminal_id.is_nil()
            && !begin.runtime_epoch.is_nil()
            && !begin.invocation_id.is_nil()
            && !begin.hook_token.is_nil()
            && !begin.helper_capability.is_nil()
            && valid_identity(begin.worker)
            && valid_identity(begin.process)
            && !begin.handshake_boot_id.is_empty()
            && begin.handshake_deadline_ns != 0
            && canonical_session_id(&begin.provider_session_id)
            && valid_absolute(&begin.cwd)
            && valid_absolute(&begin.data_home),
        AUTHORITY_BEGIN_INVALID
    );
    Ok(())
}

pub(super) fn validate_event(event: &GenerationEvent, data_home: &Path) -> Result<()> {
    let valid = match event {
        GenerationEvent::SessionStart {
            event_id,
            provider_session_id,
            cwd,
            transcript_path,
            state,
            ..
        } => {
            valid_event_identity(*event_id, provider_session_id)
                && valid_active_state(*state)
                && valid_absolute(cwd)
                && valid_transcript(data_home, transcript_path, provider_session_id)
        }
        GenerationEvent::StateChanged {
            event_id,
            provider_session_id,
            state,
            ..
        } => valid_event_identity(*event_id, provider_session_id) && valid_active_state(*state),
        GenerationEvent::CwdChanged {
            event_id,
            provider_session_id,
            cwd,
            transcript_path,
        } => {
            valid_event_identity(*event_id, provider_session_id)
                && valid_absolute(cwd)
                && valid_transcript(data_home, transcript_path, provider_session_id)
        }
        GenerationEvent::SessionEnd {
            event_id,
            provider_session_id,
            reason,
            boundary,
        } => {
            valid_event_identity(*event_id, provider_session_id)
                && SessionEndTermination {
                    reason: *reason,
                    boundary: *boundary,
                }
                .valid()
        }
    };
    ensure!(valid, AUTHORITY_EVENT_INVALID);
    Ok(())
}

pub(super) fn validate_authority(authority: &ClaudeGenerationAuthority) -> Result<()> {
    validate_begin(&GenerationBegin {
        terminal_id: authority.terminal_id,
        runtime_epoch: authority.runtime_epoch,
        invocation_id: authority.invocation_id,
        hook_token: authority.hook_token,
        helper_capability: authority.helper_capability,
        worker: authority.worker,
        process: authority.process,
        settings_identity: authority.settings_identity,
        handshake_boot_id: authority.handshake_boot_id.clone(),
        handshake_deadline_ns: authority.handshake_deadline_ns,
        provider_session_id: authority.provider_session_id.clone(),
        discover_session: authority.discover_session,
        mode: authority.mode,
        cwd: authority.cwd.clone(),
        data_home: authority.data_home.clone(),
    })?;
    let transcript_valid = authority.transcript_path.as_ref().is_some_and(|path| {
        valid_transcript(&authority.data_home, path, &authority.provider_session_id)
    }) && authority.transcript_identity.is_some();
    let candidate_valid = authority
        .termination_candidate
        .is_none_or(|candidate| candidate.valid() && authority.state == AgentState::Unknown);
    let state_valid = match authority.state {
        AgentState::Running | AgentState::WaitingForInput => {
            transcript_valid && authority.termination_candidate.is_none()
        }
        AgentState::Unknown => {
            authority.transcript_path.is_none()
                && authority.transcript_identity.is_none()
                && candidate_valid
        }
        AgentState::Exited => {
            (authority.transcript_path.is_none() && authority.transcript_identity.is_none())
                || transcript_valid && authority.termination_candidate.is_none()
        }
        AgentState::Active | AgentState::WaitingForPermission => false,
    };
    let mut replay = std::collections::HashSet::new();
    ensure!(
        authority.revision >= INITIAL_REVISION
            && authority.generation >= INITIAL_GENERATION
            && authority.replay_events.len() <= MAX_REPLAY_EVENT_IDS
            && authority.replay_events.iter().all(|applied| {
                applied.generation >= INITIAL_GENERATION
                    && applied.generation <= authority.generation
                    && applied.sequence >= crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE
                    && applied.sequence <= authority.last_event_sequence
                    && validate_event(&applied.event, &authority.data_home).is_ok()
                    && replay.insert(applied.sequence)
            })
            && authority
                .binding_revision
                .is_none_or(|revision| revision == authority.revision)
            && candidate_valid
            && state_valid
            && (!authority.discover_session
                || (authority.state == AgentState::Unknown && authority.transcript_path.is_none())),
        AUTHORITY_BEGIN_INVALID
    );
    Ok(())
}

pub(super) fn canonical_session_id(value: &str) -> bool {
    Uuid::parse_str(value)
        .ok()
        .is_some_and(|parsed| parsed.hyphenated().to_string() == value)
}

fn valid_event_identity(event_id: Uuid, session_id: &str) -> bool {
    !event_id.is_nil() && canonical_session_id(session_id)
}

fn valid_identity(identity: AuthorityProcessIdentity) -> bool {
    identity.pid != 0 && identity.start_time != 0
}

fn valid_active_state(state: AgentState) -> bool {
    matches!(state, AgentState::Running | AgentState::WaitingForInput)
}

fn valid_absolute(path: &Path) -> bool {
    path.is_absolute()
        && !path.as_os_str().is_empty()
        && !path
            .components()
            .any(|component| component == Component::ParentDir)
}

pub(super) fn valid_transcript(data_home: &Path, transcript: &Path, session_id: &str) -> bool {
    let Ok(relative) = transcript.strip_prefix(data_home) else {
        return false;
    };
    let parts = relative.components().collect::<Vec<_>>();
    parts.len() == TRANSCRIPT_RELATIVE_COMPONENTS
        && parts
            .first()
            .is_some_and(|part| part.as_os_str() == crate::agent::claude::constants::EVIDENCE_DIR)
        && transcript.extension().and_then(|value| value.to_str())
            == Some(crate::agent::constants::JSONL_EXTENSION)
        && transcript.file_stem().and_then(|value| value.to_str()) == Some(session_id)
}
