use super::constants::event_names;
use super::event::{EventKind, NotificationKind, SessionEndReason, SessionStartSource};
use super::identity::{HookProviderKind, ProviderVersion};

pub trait EventMapper {
    fn map(&self, event_name: &str) -> Option<EventKind>;

    fn map_for_version(&self, event_name: &str, _version: ProviderVersion) -> Option<EventKind> {
        self.map(event_name)
    }
}

pub trait ProviderEventMapper: EventMapper {
    fn provider(&self) -> HookProviderKind;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CodexEventMapper;

#[derive(Clone, Copy, Debug, Default)]
pub struct ClaudeEventMapper;

fn core_event(name: &str) -> Option<EventKind> {
    Some(match name {
        event_names::SESSION_START => EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
        event_names::SESSION_END => EventKind::SessionEnd {
            reason: SessionEndReason::Other,
        },
        event_names::USER_PROMPT_SUBMIT => EventKind::UserPromptSubmit,
        event_names::PRE_TOOL_USE => EventKind::PreToolUse,
        event_names::PERMISSION_REQUEST => EventKind::PermissionRequest,
        event_names::POST_TOOL_USE => EventKind::PostToolUse,
        event_names::PRE_COMPACT => EventKind::PreCompact,
        event_names::POST_COMPACT => EventKind::PostCompact,
        event_names::SUBAGENT_START => EventKind::SubagentStart,
        event_names::SUBAGENT_STOP => EventKind::SubagentStop,
        event_names::STOP => EventKind::Stop,
        _ => return None,
    })
}

impl EventMapper for CodexEventMapper {
    fn map(&self, event_name: &str) -> Option<EventKind> {
        core_event(event_name)
    }
}

impl ProviderEventMapper for CodexEventMapper {
    fn provider(&self) -> HookProviderKind {
        HookProviderKind::Codex
    }
}

impl EventMapper for ClaudeEventMapper {
    fn map(&self, event_name: &str) -> Option<EventKind> {
        core_event(event_name).or_else(|| match event_name {
            event_names::POST_TOOL_USE_FAILURE => Some(EventKind::PostToolUseFailure),
            event_names::STOP_FAILURE => Some(EventKind::StopFailure),
            event_names::NOTIFICATION => Some(EventKind::Notification {
                kind: NotificationKind::Other(String::new()),
            }),
            event_names::PERMISSION_DENIED => Some(EventKind::PermissionDenied),
            event_names::CWD_CHANGED => Some(EventKind::CwdChanged),
            event_names::ELICITATION => Some(EventKind::Elicitation),
            event_names::ELICITATION_RESULT => Some(EventKind::ElicitationResult),
            _ => None,
        })
    }
}

impl ProviderEventMapper for ClaudeEventMapper {
    fn provider(&self) -> HookProviderKind {
        HookProviderKind::Claude
    }
}
