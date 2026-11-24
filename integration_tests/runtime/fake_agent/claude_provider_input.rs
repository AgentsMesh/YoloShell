use crate::fake_agent_constants::*;
use serde::Serialize;
use serde_json::Value;
use std::error::Error;
use std::path::Path;
use yolo_runtime::claude_hooks as events;

const INVALID: &str = "fake Claude provider hook input is invalid";
const PERMISSION_MODE: &str = "default";
const END_REASON: &str = "other";
const TOOL_NAME: &str = "Read";
const TOOL_USE_ID: &str = "fixture-tool-use";

#[derive(Serialize)]
struct Common<'a> {
    session_id: &'a str,
    transcript_path: &'a Path,
    cwd: &'a Path,
}

#[derive(Serialize)]
#[serde(tag = "hook_event_name")]
enum Input<'a> {
    SessionStart {
        #[serde(flatten)]
        common: Common<'a>,
        source: &'a str,
    },
    SessionEnd {
        #[serde(flatten)]
        common: Common<'a>,
        reason: &'a str,
    },
    UserPromptSubmit {
        #[serde(flatten)]
        common: Common<'a>,
        permission_mode: &'a str,
        prompt: &'a str,
    },
    Stop {
        #[serde(flatten)]
        common: Common<'a>,
        permission_mode: &'a str,
        stop_hook_active: bool,
        last_assistant_message: &'a str,
    },
    CwdChanged {
        #[serde(flatten)]
        common: Common<'a>,
        old_cwd: &'a Path,
        new_cwd: &'a Path,
    },
    Notification {
        #[serde(flatten)]
        common: Common<'a>,
        message: &'a str,
        notification_type: &'a str,
    },
    PreToolUse {
        #[serde(flatten)]
        common: Common<'a>,
        permission_mode: &'a str,
        tool_name: &'a str,
        tool_input: Value,
        tool_use_id: &'a str,
    },
}

pub(crate) fn cwd_changed(
    session_id: &str,
    transcript_path: &Path,
    old_cwd: &Path,
    new_cwd: &Path,
) -> Result<Value, serde_json::Error> {
    serde_json::to_value(Input::CwdChanged {
        common: Common {
            session_id,
            transcript_path,
            cwd: new_cwd,
        },
        old_cwd,
        new_cwd,
    })
}

pub(crate) fn build<'a>(
    event: &str,
    session_id: &'a str,
    transcript_path: &'a Path,
    cwd: &'a Path,
    source: Option<&'a str>,
    notification_type: Option<&'a str>,
) -> Result<Value, Box<dyn Error>> {
    let common = Common {
        session_id,
        transcript_path,
        cwd,
    };
    let input = match event {
        events::CLAUDE_SESSION_START_EVENT => Input::SessionStart {
            common,
            source: source.ok_or(INVALID)?,
        },
        events::CLAUDE_SESSION_END_EVENT => Input::SessionEnd {
            common,
            reason: END_REASON,
        },
        events::CLAUDE_USER_PROMPT_EVENT => Input::UserPromptSubmit {
            common,
            permission_mode: PERMISSION_MODE,
            prompt: PACKAGED_REDACTED_USER_CONTENT,
        },
        events::CLAUDE_STOP_EVENT => Input::Stop {
            common,
            permission_mode: PERMISSION_MODE,
            stop_hook_active: false,
            last_assistant_message: PACKAGED_REDACTED_ASSISTANT_CONTENT,
        },
        events::CLAUDE_CWD_CHANGED_EVENT => Input::CwdChanged {
            common,
            old_cwd: cwd,
            new_cwd: cwd,
        },
        events::CLAUDE_NOTIFICATION_EVENT => Input::Notification {
            common,
            message: PACKAGED_REDACTED_ASSISTANT_CONTENT,
            notification_type: notification_type.unwrap_or(PACKAGED_WAITING_NOTIFICATION_TYPE),
        },
        events::CLAUDE_PRE_TOOL_USE_EVENT => Input::PreToolUse {
            common,
            permission_mode: PERMISSION_MODE,
            tool_name: TOOL_NAME,
            tool_input: serde_json::json!({PROVIDER_TOOL_PATH_FIELD: transcript_path}),
            tool_use_id: TOOL_USE_ID,
        },
        _ => return Err(INVALID.into()),
    };
    Ok(serde_json::to_value(input)?)
}
