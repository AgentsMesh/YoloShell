use super::SessionEndTermination;
use super::constants::*;
use super::types::{
    AppliedGenerationEvent, ClaudeGenerationAuthority, GenerationApplyDisposition, GenerationEvent,
    InvocationMode, SessionStartSource,
};
use super::validation::validate_event;
use crate::agent::AgentState;
use crate::paths::identity::ObjectIdentity;
use anyhow::{Context, Result};

pub(super) fn preview(
    current: &ClaudeGenerationAuthority,
    sequence: u64,
    event: &GenerationEvent,
) -> Result<GenerationApplyDisposition> {
    validate_event(event, &current.data_home)?;
    if let Some(applied) = current
        .replay_events
        .iter()
        .find(|item| item.sequence == sequence)
    {
        if applied.event != *event {
            return Ok(GenerationApplyDisposition::Invalidated);
        }
        if applied.generation == current.generation {
            return Ok(GenerationApplyDisposition::Replay);
        }
        return Ok(GenerationApplyDisposition::Stale);
    }
    if sequence <= current.last_event_sequence {
        return Ok(GenerationApplyDisposition::Stale);
    }
    if current.last_event_sequence.checked_add(1) != Some(sequence) {
        return Ok(GenerationApplyDisposition::Invalidated);
    }
    Ok(match event {
        GenerationEvent::SessionStart {
            provider_session_id,
            source,
            ..
        } => preview_start(current, provider_session_id, *source),
        GenerationEvent::StateChanged {
            provider_session_id, ..
        }
        | GenerationEvent::CwdChanged {
            provider_session_id, ..
        }
        | GenerationEvent::SessionEnd {
            provider_session_id, ..
        } => preview_existing(current, provider_session_id),
    })
}

pub(super) fn apply(
    current: &ClaudeGenerationAuthority,
    sequence: u64,
    event: &GenerationEvent,
    transcript_identity: Option<ObjectIdentity>,
) -> Result<(ClaudeGenerationAuthority, GenerationApplyDisposition)> {
    let disposition = preview(current, sequence, event)?;
    if disposition != GenerationApplyDisposition::Applied {
        return Ok((current.clone(), disposition));
    }
    let mut next = match event {
        GenerationEvent::StateChanged { state, .. } => {
            super::transition_mutation::advance_state(current, *state)?
        }
        GenerationEvent::SessionEnd {
            reason, boundary, ..
        } => super::transition_mutation::session_end(
            current,
            SessionEndTermination {
                reason: *reason,
                boundary: *boundary,
            },
        )?,
        GenerationEvent::SessionStart { .. } | GenerationEvent::CwdChanged { .. } => {
            super::transition_mutation::advance(current)?
        }
    };
    next.last_event_sequence = sequence;
    match event {
        GenerationEvent::SessionStart {
            provider_session_id,
            cwd,
            transcript_path,
            state,
            ..
        } => super::transition_mutation::start(
            &mut next,
            provider_session_id,
            cwd,
            transcript_path,
            *state,
            transcript_identity.context(AUTHORITY_TRANSCRIPT_IDENTITY_MISSING)?,
        )?,
        GenerationEvent::StateChanged { .. } => {}
        GenerationEvent::CwdChanged {
            cwd,
            transcript_path,
            ..
        } => super::transition_mutation::cwd(
            &mut next,
            cwd,
            transcript_path,
            transcript_identity.context(AUTHORITY_TRANSCRIPT_IDENTITY_MISSING)?,
        )?,
        GenerationEvent::SessionEnd { .. } => {}
    }
    remember_event(&mut next, sequence, event.clone());
    Ok((next, disposition))
}

fn preview_start(
    authority: &ClaudeGenerationAuthority,
    session_id: &str,
    source: SessionStartSource,
) -> GenerationApplyDisposition {
    let same_session = authority.provider_session_id == session_id;
    let accepted = if same_session {
        match authority.state {
            AgentState::Unknown => authority.termination_candidate.map_or_else(
                || invocation_source_matches(authority.mode, source),
                |candidate| candidate.accepts_start(source),
            ),
            AgentState::Running | AgentState::WaitingForInput => {
                source == SessionStartSource::Compact
            }
            AgentState::Exited => matches!(
                source,
                SessionStartSource::Resume | SessionStartSource::Fork
            ),
            AgentState::Active | AgentState::WaitingForPermission => false,
        }
    } else if authority.state == AgentState::Unknown {
        authority.termination_candidate.map_or_else(
            || authority.discover_session && invocation_source_matches(authority.mode, source),
            |candidate| candidate.accepts_start(source),
        )
    } else {
        matches!(
            authority.state,
            AgentState::Running | AgentState::WaitingForInput | AgentState::Exited
        ) && matches!(
            source,
            SessionStartSource::Resume | SessionStartSource::Clear | SessionStartSource::Fork
        )
    };
    if accepted {
        GenerationApplyDisposition::Applied
    } else {
        GenerationApplyDisposition::Invalidated
    }
}

fn preview_existing(
    authority: &ClaudeGenerationAuthority,
    session_id: &str,
) -> GenerationApplyDisposition {
    if authority.provider_session_id != session_id {
        return GenerationApplyDisposition::Stale;
    }
    if matches!(
        authority.state,
        AgentState::Running | AgentState::WaitingForInput
    ) {
        GenerationApplyDisposition::Applied
    } else {
        GenerationApplyDisposition::Invalidated
    }
}

fn invocation_source_matches(mode: InvocationMode, source: SessionStartSource) -> bool {
    matches!(
        (mode, source),
        (InvocationMode::Fresh, SessionStartSource::Fresh)
            | (
                InvocationMode::Resume,
                SessionStartSource::Resume | SessionStartSource::Fork
            )
    )
}

fn remember_event(
    authority: &mut ClaudeGenerationAuthority,
    sequence: u64,
    event: GenerationEvent,
) {
    if authority.replay_events.len() == MAX_REPLAY_EVENT_IDS {
        authority.replay_events.pop_front();
    }
    authority.replay_events.push_back(AppliedGenerationEvent {
        sequence,
        generation: authority.generation,
        event,
    });
}
