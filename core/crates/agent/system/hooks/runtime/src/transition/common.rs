use super::helpers::notification_attention;
use crate::event::{EventKind, HookEnvelope};
use crate::state::{AttentionState, CompactionPhase, StopCandidate, TurnState};

pub(super) fn apply_common(
    event: &EventKind,
    envelope: &HookEnvelope,
    turn: &mut TurnState,
    attention: &mut AttentionState,
    active_tools: &mut std::collections::BTreeSet<String>,
    stop_candidate: &mut Option<StopCandidate>,
    compaction: &mut Option<CompactionPhase>,
) {
    let tool = envelope.tool_use_id.clone().or_else(|| envelope.tool_name.clone());
    match event {
        EventKind::UserPromptSubmit => {
            *turn = TurnState::Running;
            *attention = AttentionState::None;
            *stop_candidate = None;
        }
        EventKind::PreToolUse => {
            *turn = TurnState::Running;
            *attention = AttentionState::None;
            if let Some(tool) = tool {
                active_tools.insert(tool);
            }
        }
        EventKind::PostToolUse | EventKind::PostToolUseFailure => {
            if let Some(tool) = tool {
                active_tools.remove(&tool);
            }
            *turn = TurnState::Running;
            *attention = AttentionState::None;
        }
        EventKind::TurnEnded => {
            *turn = TurnState::Idle;
            *attention = AttentionState::None;
            *stop_candidate = None;
            active_tools.clear();
        }
        EventKind::PermissionRequest => {
            *turn = TurnState::Running;
            *attention = AttentionState::PermissionCandidate;
        }
        EventKind::PermissionDenied => { *turn = TurnState::Running; *attention = AttentionState::None; }
        EventKind::Notification { kind } => *attention = notification_attention(kind),
        EventKind::Elicitation => *attention = AttentionState::Elicitation,
        EventKind::ElicitationResult => {
            *attention = AttentionState::None;
            *turn = TurnState::Running;
        }
        EventKind::PreCompact => {
            *compaction = Some(CompactionPhase::begin(*turn, envelope.sequence));
            *turn = TurnState::Compacting;
        }
        EventKind::PostCompact => {
            if let Some(phase) = compaction.take() {
                *turn = phase.prior_turn();
            }
        }
        EventKind::Stop => {
            *stop_candidate = Some(StopCandidate {
                event_id: envelope.event_id,
                turn_id: envelope.turn_id.clone(),
                continued: envelope.stop_hook_active.unwrap_or(false),
            });
            *turn = TurnState::StopCandidate;
            *attention = AttentionState::None;
            active_tools.clear();
        }
        EventKind::StopFailure => {
            *turn = TurnState::Failed;
            *attention = AttentionState::None;
            active_tools.clear();
            *stop_candidate = None;
        }
        EventKind::SessionEnd { .. } => {
            *turn = TurnState::Unknown;
            *attention = AttentionState::None;
            *stop_candidate = None;
            active_tools.clear();
        }
        EventKind::CwdChanged
        | EventKind::SubagentStart
        | EventKind::SubagentStop
        | EventKind::SessionStart { .. } => {}
    }
    if !matches!(
        event,
        EventKind::PreCompact | EventKind::Notification { .. } | EventKind::CwdChanged
    ) { *compaction = None; }
}
