use super::constants::*;
use super::termination::{SessionEndBoundary, SessionEndTermination};
use super::types::ClaudeGenerationAuthority;
use crate::agent::AgentState;
use crate::paths::identity::ObjectIdentity;
use anyhow::{Context, Result, ensure};

pub(super) fn advance(authority: &ClaudeGenerationAuthority) -> Result<ClaudeGenerationAuthority> {
    let mut next = authority.clone();
    next.revision = next
        .revision
        .checked_add(REVISION_INCREMENT)
        .context(AUTHORITY_REVISION_OVERFLOW)?;
    next.binding_revision = None;
    Ok(next)
}

pub(super) fn advance_state(
    authority: &ClaudeGenerationAuthority,
    state: AgentState,
) -> Result<ClaudeGenerationAuthority> {
    let mut next = advance(authority)?;
    next.state = state;
    Ok(next)
}

pub(super) fn session_end(
    authority: &ClaudeGenerationAuthority,
    termination: SessionEndTermination,
) -> Result<ClaudeGenerationAuthority> {
    ensure!(termination.valid(), AUTHORITY_EVENT_INVALID);
    let mut next = advance_state(authority, AgentState::Unknown)?;
    next.transcript_path = None;
    next.transcript_identity = None;
    next.termination_candidate = Some(termination);
    Ok(next)
}

pub(super) fn process_exit(
    authority: &ClaudeGenerationAuthority,
    process: super::AuthorityProcessIdentity,
) -> Result<ClaudeGenerationAuthority> {
    ensure!(authority.process == process, PROCESS_EXIT_IDENTITY_MISMATCH);
    ensure!(
        authority.state == AgentState::Unknown
            && authority.transcript_path.is_none()
            && authority.transcript_identity.is_none()
            && authority.termination_candidate.is_some_and(|candidate| {
                candidate.valid() && candidate.boundary == SessionEndBoundary::FinalCandidate
            }),
        PROCESS_EXIT_NOT_FINAL
    );
    let mut next = advance_state(authority, AgentState::Exited)?;
    next.termination_candidate = None;
    Ok(next)
}

pub(super) fn start(
    authority: &mut ClaudeGenerationAuthority,
    session_id: &str,
    cwd: &std::path::Path,
    transcript: &std::path::Path,
    state: AgentState,
    identity: ObjectIdentity,
) -> Result<()> {
    let discovered = authority.discover_session && authority.state == AgentState::Unknown;
    let starts_new_generation = authority.provider_session_id != session_id
        || authority.state == AgentState::Exited
        || authority.termination_candidate.is_some();
    if starts_new_generation && !discovered {
        authority.generation = authority
            .generation
            .checked_add(GENERATION_INCREMENT)
            .context(AUTHORITY_GENERATION_OVERFLOW)?;
    }
    if authority.provider_session_id == session_id && authority.transcript_identity.is_some() {
        ensure!(
            authority.transcript_path.as_deref() == Some(transcript)
                && authority.transcript_identity == Some(identity),
            AUTHORITY_TRANSCRIPT_IDENTITY_MISMATCH
        );
    }
    authority.provider_session_id = session_id.to_owned();
    authority.discover_session = false;
    authority.cwd = cwd.to_path_buf();
    authority.transcript_path = Some(transcript.to_path_buf());
    authority.transcript_identity = Some(identity);
    authority.state = state;
    authority.termination_candidate = None;
    Ok(())
}

pub(super) fn cwd(
    authority: &mut ClaudeGenerationAuthority,
    cwd: &std::path::Path,
    transcript: &std::path::Path,
    identity: ObjectIdentity,
) -> Result<()> {
    ensure!(
        authority.transcript_path.as_deref() == Some(transcript)
            && authority.transcript_identity == Some(identity),
        AUTHORITY_TRANSCRIPT_IDENTITY_MISMATCH
    );
    authority.cwd = cwd.to_path_buf();
    Ok(())
}
