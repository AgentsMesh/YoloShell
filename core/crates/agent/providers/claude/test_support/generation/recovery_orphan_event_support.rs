use crate::claude_hooks::{ClaudeHookState, ClaudeLifecycleEvent};

pub(super) fn hook_event(event: &super::super::super::GenerationEvent) -> ClaudeLifecycleEvent {
    let super::super::super::GenerationEvent::SessionStart {
        event_id,
        provider_session_id,
        source,
        cwd,
        transcript_path,
        state,
    } = event
    else {
        unreachable!();
    };
    ClaudeLifecycleEvent::SessionStart {
        event_id: *event_id,
        provider_session_id: provider_session_id.clone(),
        source: match source {
            super::super::super::SessionStartSource::Fresh => {
                crate::claude_hooks::ClaudeSessionStartSource::Fresh
            }
            _ => unreachable!(),
        },
        cwd: cwd.clone(),
        transcript_path: transcript_path.clone(),
        state: match state {
            crate::agent::AgentState::Running => ClaudeHookState::Running,
            _ => unreachable!(),
        },
    }
}
