use super::constants::*;
use super::types::{AuthorityAuthentication, ClaudeGenerationAuthority, GenerationBegin};
use super::validation::{validate_authority, validate_begin};
use crate::agent::{AgentBinding, AgentKind, AgentState};
use anyhow::{Result, ensure};

pub(super) fn begin(
    current: Option<&ClaudeGenerationAuthority>,
    begin: &GenerationBegin,
) -> Result<ClaudeGenerationAuthority> {
    validate_begin(begin)?;
    if let Some(current) = current {
        validate_loaded(current)?;
        ensure!(
            current.terminal_id == begin.terminal_id,
            AUTHORITY_SESSION_MISMATCH
        );
    }
    let revision = next_value(
        current.map(|value| value.revision),
        INITIAL_REVISION,
        AUTHORITY_REVISION_OVERFLOW,
    )?;
    let generation = next_value(
        current.map(|value| value.generation),
        INITIAL_GENERATION,
        AUTHORITY_GENERATION_OVERFLOW,
    )?;
    Ok(ClaudeGenerationAuthority {
        schema_version: AUTHORITY_SCHEMA_VERSION,
        terminal_id: begin.terminal_id,
        runtime_epoch: begin.runtime_epoch,
        invocation_id: begin.invocation_id,
        hook_token: begin.hook_token,
        helper_capability: begin.helper_capability,
        worker: begin.worker,
        process: begin.process,
        settings_identity: begin.settings_identity,
        handshake_boot_id: begin.handshake_boot_id.clone(),
        handshake_deadline_ns: begin.handshake_deadline_ns,
        revision,
        generation,
        last_event_sequence: INITIAL_EVENT_SEQUENCE,
        provider_session_id: begin.provider_session_id.clone(),
        discover_session: begin.discover_session,
        mode: begin.mode,
        cwd: begin.cwd.clone(),
        data_home: begin.data_home.clone(),
        transcript_path: None,
        transcript_identity: None,
        binding_revision: None,
        state: AgentState::Unknown,
        termination_candidate: None,
        replay_events: Default::default(),
    })
}

pub(super) fn validate_loaded(authority: &ClaudeGenerationAuthority) -> Result<()> {
    ensure!(
        authority.schema_version == AUTHORITY_SCHEMA_VERSION,
        AUTHORITY_SCHEMA_MISMATCH
    );
    validate_authority(authority)
}

pub(super) fn authenticated(
    authority: &ClaudeGenerationAuthority,
    authentication: AuthorityAuthentication,
) -> bool {
    authority.terminal_id == authentication.terminal_id
        && authority.runtime_epoch == authentication.runtime_epoch
        && authority.invocation_id == authentication.invocation_id
        && authority.hook_token == authentication.hook_token
        && authority.helper_capability == authentication.helper_capability
        && authority.process == authentication.process
}

pub(super) fn matches_begin(
    authority: &ClaudeGenerationAuthority,
    begin: &GenerationBegin,
) -> bool {
    authority.terminal_id == begin.terminal_id
        && authority.runtime_epoch == begin.runtime_epoch
        && authority.invocation_id == begin.invocation_id
        && authority.hook_token == begin.hook_token
        && authority.helper_capability == begin.helper_capability
        && authority.worker == begin.worker
        && authority.process == begin.process
        && authority.settings_identity == begin.settings_identity
        && authority.provider_session_id == begin.provider_session_id
        && authority.discover_session == begin.discover_session
        && authority.mode == begin.mode
        && authority.cwd == begin.cwd
        && authority.data_home == begin.data_home
        && authority.transcript_path.is_none()
        && authority.transcript_identity.is_none()
        && authority.binding_revision.is_none()
        && authority.last_event_sequence == INITIAL_EVENT_SEQUENCE
        && authority.state == AgentState::Unknown
        && authority.termination_candidate.is_none()
}

pub(super) fn matches_binding(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
) -> bool {
    authority.binding_revision == Some(authority.revision)
        && matches_binding_payload(authority, binding)
}

pub(super) fn matches_binding_payload(
    authority: &ClaudeGenerationAuthority,
    binding: &AgentBinding,
) -> bool {
    if authority.discover_session
        || binding.schema_version != crate::agent::constants::BINDING_SCHEMA_VERSION
        || binding.kind != AgentKind::ClaudeCode
        || binding.terminal_id != authority.terminal_id
        || binding.provider_session_id != authority.provider_session_id
        || binding.executable != crate::agent::claude::constants::EXECUTABLE
        || binding.cwd != authority.cwd.to_string_lossy()
        || binding.data_home != authority.data_home.to_string_lossy()
        || binding.state != authority.state
        || binding.recovery_availability != crate::agent::AgentRecoveryAvailability::Available
    {
        return false;
    }
    let evidence = std::path::Path::new(&binding.evidence_path);
    let evidence_matches =
        match authority.state {
            AgentState::Running | AgentState::WaitingForInput => authority
                .transcript_path
                .as_ref()
                .is_some_and(|path| evidence == path && authority.transcript_identity_is_current()),
            AgentState::Exited => {
                authority.transcript_path.as_ref().is_some_and(|path| {
                    evidence == path && authority.transcript_identity_is_current()
                }) || authority.transcript_path.is_none()
                    && super::validation::valid_transcript(
                        &authority.data_home,
                        evidence,
                        &authority.provider_session_id,
                    )
                    && super::super::authoritative_evidence::named_transcript_identity(
                        &authority.data_home,
                        evidence,
                    )
                    .ok()
                    .flatten()
                    .is_some()
            }
            AgentState::Active | AgentState::Unknown | AgentState::WaitingForPermission => false,
        };
    evidence_matches
        && binding.process_pid == active(authority.state).then_some(authority.process.pid)
        && binding.process_start_time
            == active(authority.state).then_some(authority.process.start_time)
}

fn active(state: AgentState) -> bool {
    matches!(state, AgentState::Running | AgentState::WaitingForInput)
}

fn next_value(current: Option<u64>, initial: u64, overflow: &'static str) -> Result<u64> {
    current.map_or(Ok(initial), |value| {
        value
            .checked_add(REVISION_INCREMENT)
            .ok_or_else(|| anyhow::anyhow!(overflow))
    })
}
