use crate::constants::*;
use crate::payload::HookPayload;
use anyhow::Result;
use yolo_hooks_runtime::{EventKind, NotificationKind, SessionEndReason, SessionStartSource};

/// Map Grok's event name to the canonical lifecycle fact. Unknown events are
/// telemetry and decode to no fact.
pub(super) fn event(payload: &HookPayload) -> Result<Option<EventKind>> {
    let name = normalize_event_name(
        payload
            .text("hookEventName", PAYLOAD_HOOK_EVENT_NAME)
            .ok_or_else(|| anyhow::anyhow!(INVALID_PAYLOAD))?,
    );
    Ok(Some(match name.as_str() {
        EVENT_SESSION_START_SNAKE => EventKind::SessionStart {
            source: start_source(payload.text("source", "source")),
        },
        EVENT_SESSION_END_SNAKE => EventKind::SessionEnd {
            reason: end_reason(payload.text("reason", "reason")),
        },
        EVENT_USER_PROMPT_SUBMIT_SNAKE => EventKind::UserPromptSubmit,
        // A genuine completion ends the turn, and an interrupt (StopCancelled)
        // ends it too. A shutdown stop follows SessionEnd, which already ends
        // the session, so it is not a second lifecycle fact.
        EVENT_STOP_SNAKE => {
            if payload
                .text("reason", "reason")
                .is_some_and(|reason| reason.eq_ignore_ascii_case(REASON_SHUTDOWN))
            {
                return Ok(None);
            }
            EventKind::TurnEnded
        }
        EVENT_STOP_CANCELLED_SNAKE => EventKind::TurnEnded,
        EVENT_STOP_FAILURE_SNAKE => EventKind::StopFailure,
        EVENT_PRE_TOOL_USE_SNAKE => EventKind::PreToolUse,
        EVENT_POST_TOOL_USE_SNAKE => EventKind::PostToolUse,
        EVENT_POST_TOOL_USE_FAILURE_SNAKE => EventKind::PostToolUseFailure,
        "permission_denied" => EventKind::PermissionDenied,
        EVENT_NOTIFICATION_SNAKE => EventKind::Notification {
            kind: notification_kind(payload.text("notificationType", PAYLOAD_NOTIFICATION_TYPE)),
        },
        EVENT_PRE_COMPACT_SNAKE => EventKind::PreCompact,
        EVENT_POST_COMPACT_SNAKE => EventKind::PostCompact,
        EVENT_SUBAGENT_START_SNAKE => EventKind::SubagentStart,
        EVENT_SUBAGENT_STOP_SNAKE => EventKind::SubagentStop,
        _ => return Ok(None),
    }))
}

/// `hookEventName` carries grok's snake_case value; `hook_event_name` carries
/// Claude's PascalCase spelling. Both normalize to the snake_case name.
fn normalize_event_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len() + EVENT_NAME_CAPACITY_MARGIN);
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 {
                normalized.push('_');
            }
            normalized.push(character.to_ascii_lowercase());
        } else {
            normalized.push(character);
        }
    }
    normalized
}

/// Grok reports a resumed session as `load`; `new` is a fresh session. The
/// source must be `Resume` so the hook-generation store inherits the durable
/// binding origin (including `detected_at_ms`) that the Host resume
/// transaction compares against its planned binding.
fn start_source(source: Option<&str>) -> SessionStartSource {
    match source {
        Some(value) if value.eq_ignore_ascii_case("load") => SessionStartSource::Resume,
        Some(value) if value.eq_ignore_ascii_case("resume") => SessionStartSource::Resume,
        Some(value) if value.eq_ignore_ascii_case("clear") => SessionStartSource::Clear,
        Some(value) if value.eq_ignore_ascii_case("compact") => SessionStartSource::Compact,
        Some(value) if value.eq_ignore_ascii_case("fork") => SessionStartSource::Fork,
        _ => SessionStartSource::Startup,
    }
}

fn end_reason(reason: Option<&str>) -> SessionEndReason {
    match reason {
        Some(value) if value.eq_ignore_ascii_case("clear") => SessionEndReason::Clear,
        Some(value) if value.eq_ignore_ascii_case("resume") => SessionEndReason::Resume,
        _ => SessionEndReason::Other,
    }
}

fn notification_kind(kind: Option<&str>) -> NotificationKind {
    match kind {
        Some(value) if value.eq_ignore_ascii_case(NOTIFICATION_PERMISSION_PROMPT) => {
            NotificationKind::PermissionPrompt
        }
        Some(value)
            if value.eq_ignore_ascii_case(NOTIFICATION_IDLE_PROMPT)
                || value.eq_ignore_ascii_case(NOTIFICATION_AGENT_NEEDS_INPUT) =>
        {
            NotificationKind::AgentNeedsInput
        }
        Some(value) if value.eq_ignore_ascii_case(NOTIFICATION_ELICITATION) => {
            NotificationKind::Elicitation
        }
        Some(value) => NotificationKind::Other(value.to_owned()),
        None => NotificationKind::Other(String::new()),
    }
}
