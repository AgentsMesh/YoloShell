use super::wire::CodexHookInput;
use std::path::Path;

pub(super) struct CodexHookCommon<'a> {
    pub(super) session_id: &'a str,
    pub(super) transcript_path: Option<&'a Path>,
    pub(super) cwd: &'a Path,
}

impl CodexHookInput {
    pub(super) fn common(&self) -> CodexHookCommon<'_> {
        let (session_id, transcript_path, cwd) = match self {
            Self::SessionStart {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::SessionEnd {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::UserPromptSubmit {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::PreToolUse {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::PermissionRequest {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::PostToolUse {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::PreCompact {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::PostCompact {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::SubagentStart {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::SubagentStop {
                session_id,
                transcript_path,
                cwd,
                ..
            }
            | Self::Stop {
                session_id,
                transcript_path,
                cwd,
                ..
            } => (session_id, transcript_path, cwd),
        };
        CodexHookCommon {
            session_id,
            transcript_path: transcript_path.as_deref(),
            cwd,
        }
    }

    pub(super) fn turn_id(&self) -> Option<&str> {
        match self {
            Self::SessionStart { .. } | Self::SessionEnd { .. } => None,
            Self::UserPromptSubmit { turn_id, .. }
            | Self::PreToolUse { turn_id, .. }
            | Self::PermissionRequest { turn_id, .. }
            | Self::PostToolUse { turn_id, .. }
            | Self::PreCompact { turn_id, .. }
            | Self::PostCompact { turn_id, .. }
            | Self::SubagentStart { turn_id, .. }
            | Self::SubagentStop { turn_id, .. }
            | Self::Stop { turn_id, .. } => Some(turn_id),
        }
    }

    pub(super) fn agent_id(&self) -> Option<&str> {
        match self {
            Self::UserPromptSubmit { agent_id, .. }
            | Self::PreToolUse { agent_id, .. }
            | Self::PermissionRequest { agent_id, .. }
            | Self::PostToolUse { agent_id, .. }
            | Self::PreCompact { agent_id, .. }
            | Self::PostCompact { agent_id, .. } => agent_id.as_deref(),
            Self::SubagentStart { agent_id, .. } | Self::SubagentStop { agent_id, .. } => {
                Some(agent_id)
            }
            _ => None,
        }
    }

    pub(super) fn agent_type(&self) -> Option<&str> {
        match self {
            Self::UserPromptSubmit { agent_type, .. }
            | Self::PreToolUse { agent_type, .. }
            | Self::PermissionRequest { agent_type, .. }
            | Self::PostToolUse { agent_type, .. }
            | Self::PreCompact { agent_type, .. }
            | Self::PostCompact { agent_type, .. } => agent_type.as_deref(),
            Self::SubagentStart { agent_type, .. } | Self::SubagentStop { agent_type, .. } => {
                Some(agent_type)
            }
            _ => None,
        }
    }

    pub(super) fn tool_name(&self) -> Option<&str> {
        match self {
            Self::PreToolUse { tool_name, .. }
            | Self::PermissionRequest { tool_name, .. }
            | Self::PostToolUse { tool_name, .. } => Some(tool_name),
            _ => None,
        }
    }

    pub(super) fn tool_use_id(&self) -> Option<&str> {
        match self {
            Self::PreToolUse { tool_use_id, .. } | Self::PostToolUse { tool_use_id, .. } => {
                Some(tool_use_id)
            }
            _ => None,
        }
    }

    pub(super) fn stop_hook_active(&self) -> Option<bool> {
        match self {
            Self::SubagentStop {
                stop_hook_active, ..
            }
            | Self::Stop {
                stop_hook_active, ..
            } => Some(*stop_hook_active),
            _ => None,
        }
    }
}
