use super::constants::*;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct QwenHookPayload {
    pub session_id: String,
    pub transcript_path: PathBuf,
    pub cwd: PathBuf,
    pub hook_event_name: String,
    #[serde(default)]
    pub prompt_id: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub notification_type: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub tool_call_id: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub subagent_id: Option<String>,
    #[serde(default)]
    pub stop_hook_active: Option<bool>,
    #[serde(default)]
    pub is_final: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_json::Value>,
}

impl QwenHookPayload {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.session_id.is_empty()
                && self.session_id.len() <= MAX_IDENTIFIER_BYTES
                && self.cwd.is_absolute()
                && self.transcript_path.is_absolute()
                && !self.hook_event_name.is_empty(),
            INVALID
        );
        Ok(())
    }

    pub fn tool_use_id(&self) -> Option<&str> {
        self.tool_use_id
            .as_deref()
            .or(self.tool_call_id.as_deref())
    }

    pub fn agent_id(&self) -> Option<&str> {
        self.agent_id
            .as_deref()
            .or(self.subagent_id.as_deref())
    }

    pub fn event(&self) -> Result<Option<QwenHookEvent>> {
        let event = match self.hook_event_name.as_str() {
            EVENT_SESSION_START => QwenHookEvent::SessionStart {
                source: match self.source.as_deref() {
                    Some(SOURCE_STARTUP) | None => QwenSessionStartSource::Startup,
                    Some(SOURCE_RESUME) => QwenSessionStartSource::Resume,
                    Some(SOURCE_CLEAR) => QwenSessionStartSource::Clear,
                    Some(SOURCE_COMPACT) => QwenSessionStartSource::Compact,
                    Some(SOURCE_FORK) => QwenSessionStartSource::Fork,
                    Some(_) => anyhow::bail!(INVALID),
                },
            },
            EVENT_SESSION_END | EVENT_SESSION_DELETE => QwenHookEvent::SessionEnd {
                reason: match self.reason.as_deref() {
                    Some(REASON_CLEAR) => QwenSessionEndReason::Clear,
                    Some(REASON_RESUME) => QwenSessionEndReason::Resume,
                    _ => QwenSessionEndReason::Other,
                },
            },
            EVENT_USER_PROMPT_SUBMIT => QwenHookEvent::UserPromptSubmit,
            EVENT_PRE_TOOL_USE => QwenHookEvent::PreToolUse,
            EVENT_POST_TOOL_USE => QwenHookEvent::PostToolUse,
            EVENT_POST_TOOL_USE_FAILURE => QwenHookEvent::PostToolUseFailure,
            EVENT_PERMISSION_REQUEST => QwenHookEvent::PermissionRequest,
            EVENT_PERMISSION_DENIED => QwenHookEvent::PermissionDenied,
            EVENT_PRE_COMPACT => QwenHookEvent::PreCompact,
            EVENT_POST_COMPACT => QwenHookEvent::PostCompact,
            EVENT_SUBAGENT_START => QwenHookEvent::SubagentStart,
            EVENT_SUBAGENT_STOP => QwenHookEvent::SubagentStop,
            EVENT_NOTIFICATION => match self.notification_type.as_deref() {
                Some(NOTIFICATION_IDLE_PROMPT) => QwenHookEvent::TurnEnded,
                Some(NOTIFICATION_PERMISSION_PROMPT) => QwenHookEvent::NotificationPermission,
                _ => return Ok(None),
            },
            EVENT_STOP | EVENT_MESSAGE_DISPLAY | EVENT_INSTRUCTIONS_LOADED | EVENT_POST_TOOL_BATCH => {
                return Ok(None);
            }
            _ => anyhow::bail!(INVALID),
        };
        if matches!(
            event,
            QwenHookEvent::SubagentStart | QwenHookEvent::SubagentStop
        ) {
            ensure!(
                self.agent_id()
                    .is_some_and(|value| !value.is_empty()
                        && value.len() <= MAX_IDENTIFIER_BYTES),
                INVALID
            );
        }
        Ok(Some(event))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QwenSessionStartSource {
    Startup,
    Resume,
    Clear,
    Compact,
    Fork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QwenSessionEndReason {
    Other,
    Clear,
    Resume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QwenHookEvent {
    SessionStart { source: QwenSessionStartSource },
    SessionEnd { reason: QwenSessionEndReason },
    UserPromptSubmit,
    PreToolUse,
    PostToolUse,
    PostToolUseFailure,
    PermissionRequest,
    PermissionDenied,
    PreCompact,
    PostCompact,
    SubagentStart,
    SubagentStop,
    TurnEnded,
    NotificationPermission,
}
