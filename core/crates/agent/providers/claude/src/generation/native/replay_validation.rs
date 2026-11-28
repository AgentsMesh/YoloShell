use super::{AgentState, ClaudeNativeGeneration, INVALID};
use crate::agent::AgentRecoveryAvailability;
use crate::claude_hooks::{ClaudeHookState, ClaudeLifecycleEvent, ClaudeSessionEndReason};
use anyhow::{Context, Result, ensure};

pub(super) fn validate(record: &ClaudeNativeGeneration) -> Result<()> {
    let last = &record.replay_events.back().context(INVALID)?.event;
    ensure!(
        last.provider_session_id() == record.binding.provider_session_id,
        INVALID
    );
    let ended = matches!(
        last,
        ClaudeLifecycleEvent::SessionEnd {
            reason: ClaudeSessionEndReason::Other,
            ..
        }
    );
    ensure!(record.final_session_end == ended, INVALID);
    if record.revoked {
        ensure!(
            record.activity == AgentState::Unknown
                && record.binding.recovery_availability == AgentRecoveryAvailability::Unavailable,
            INVALID
        );
        return Ok(());
    }
    ensure!(
        record.binding.recovery_availability == AgentRecoveryAvailability::Available,
        INVALID
    );
    let activity = record
        .replay_events
        .iter()
        .rev()
        .find_map(|request| match &request.event {
            ClaudeLifecycleEvent::SessionStart { state, .. }
            | ClaudeLifecycleEvent::StateChanged { state, .. } => Some(match state {
                ClaudeHookState::Running => AgentState::Running,
                ClaudeHookState::WaitingForInput => AgentState::WaitingForInput,
            }),
            ClaudeLifecycleEvent::SessionEnd { .. } => Some(AgentState::Unknown),
            ClaudeLifecycleEvent::CwdChanged { .. }
            | ClaudeLifecycleEvent::InteractionRequested { .. } => None,
        });
    if let Some(expected) = activity {
        ensure!(record.activity == expected, INVALID);
    } else {
        // A bounded replay window can contain only active cwd updates.
        ensure!(
            matches!(
                record.activity,
                AgentState::Running | AgentState::WaitingForInput
            ),
            INVALID
        );
    }
    Ok(())
}
