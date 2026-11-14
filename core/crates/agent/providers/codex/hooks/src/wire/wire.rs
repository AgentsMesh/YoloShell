use super::wire_nullable::nonnull_string;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "hook_event_name", deny_unknown_fields)]
pub enum CodexHookInput {
    SessionStart {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        source: String,
    },
    SessionEnd {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        reason: String,
    },
    UserPromptSubmit {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        turn_id: String,
        prompt: String,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PreToolUse {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        turn_id: String,
        tool_name: String,
        tool_use_id: String,
        tool_input: serde_json::Value,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PermissionRequest {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        turn_id: String,
        tool_name: String,
        tool_input: serde_json::Value,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PostToolUse {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        turn_id: String,
        tool_name: String,
        tool_use_id: String,
        tool_input: serde_json::Value,
        tool_response: serde_json::Value,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PreCompact {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        turn_id: String,
        trigger: String,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    PostCompact {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        turn_id: String,
        trigger: String,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<String>,
        #[serde(default, deserialize_with = "nonnull_string")]
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_type: Option<String>,
    },
    SubagentStart {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        turn_id: String,
        agent_id: String,
        agent_type: String,
    },
    SubagentStop {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        turn_id: String,
        agent_id: String,
        agent_type: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        agent_transcript_path: Option<PathBuf>,
        stop_hook_active: bool,
        #[serde(deserialize_with = "super::wire_nullable::string")]
        last_assistant_message: Option<String>,
    },
    Stop {
        session_id: String,
        #[serde(deserialize_with = "super::wire_nullable::path")]
        transcript_path: Option<PathBuf>,
        cwd: PathBuf,
        model: String,
        permission_mode: String,
        turn_id: String,
        stop_hook_active: bool,
        #[serde(deserialize_with = "super::wire_nullable::string")]
        last_assistant_message: Option<String>,
    },
}
