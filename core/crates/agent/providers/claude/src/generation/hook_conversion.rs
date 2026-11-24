use super::{
    GenerationEvent, InvocationMode, SessionEndReason, SessionEndTermination, SessionStartSource,
};
use crate::agent::AgentState;
use crate::claude_hooks::{
    ClaudeHookState, ClaudeInvocationMode, ClaudeLifecycleEvent, ClaudeSessionStartSource,
};

pub fn invocation_mode(mode: ClaudeInvocationMode) -> InvocationMode {
    match mode {
        ClaudeInvocationMode::Fresh => InvocationMode::Fresh,
        ClaudeInvocationMode::Resume => InvocationMode::Resume,
    }
}

pub fn from_hook_event(event: &ClaudeLifecycleEvent) -> GenerationEvent {
    match event {
        ClaudeLifecycleEvent::SessionStart {
            event_id,
            provider_session_id,
            source,
            cwd,
            transcript_path,
            state,
        } => GenerationEvent::SessionStart {
            event_id: *event_id,
            provider_session_id: provider_session_id.clone(),
            source: start_source(*source),
            cwd: cwd.clone(),
            transcript_path: transcript_path.clone(),
            state: agent_state(*state),
        },
        ClaudeLifecycleEvent::StateChanged {
            event_id,
            provider_session_id,
            state,
        } => GenerationEvent::StateChanged {
            event_id: *event_id,
            provider_session_id: provider_session_id.clone(),
            state: agent_state(*state),
        },
        ClaudeLifecycleEvent::CwdChanged {
            event_id,
            provider_session_id,
            cwd,
            transcript_path,
        } => GenerationEvent::CwdChanged {
            event_id: *event_id,
            provider_session_id: provider_session_id.clone(),
            cwd: cwd.clone(),
            transcript_path: transcript_path.clone(),
        },
        ClaudeLifecycleEvent::InteractionRequested { .. } => unreachable!(
            "interaction requests are synchronous and never enter the lifecycle reducer"
        ),
        ClaudeLifecycleEvent::SessionEnd {
            event_id,
            provider_session_id,
            reason,
        } => {
            let termination = SessionEndTermination::from_reason(SessionEndReason::from(*reason));
            GenerationEvent::SessionEnd {
                event_id: *event_id,
                provider_session_id: provider_session_id.clone(),
                reason: termination.reason,
                boundary: termination.boundary,
            }
        }
    }
}

fn start_source(source: ClaudeSessionStartSource) -> SessionStartSource {
    match source {
        ClaudeSessionStartSource::Fresh => SessionStartSource::Fresh,
        ClaudeSessionStartSource::Resume => SessionStartSource::Resume,
        ClaudeSessionStartSource::Clear => SessionStartSource::Clear,
        ClaudeSessionStartSource::Compact => SessionStartSource::Compact,
        ClaudeSessionStartSource::Fork => SessionStartSource::Fork,
    }
}

fn agent_state(state: ClaudeHookState) -> AgentState {
    match state {
        ClaudeHookState::Running => AgentState::Running,
        ClaudeHookState::WaitingForInput => AgentState::WaitingForInput,
    }
}
