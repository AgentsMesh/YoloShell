use super::{CLAUDE_SESSION_END_CLEAR_REASON, CLAUDE_SESSION_END_RESUME_REASON};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeInvocationMode {
    Fresh,
    Resume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeSessionStartSource {
    Fresh,
    Resume,
    Clear,
    Compact,
    Fork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeSessionEndReason {
    Other,
    Clear,
    Resume,
}

impl ClaudeSessionEndReason {
    pub fn from_provider(value: Option<&str>) -> Self {
        match value {
            Some(CLAUDE_SESSION_END_CLEAR_REASON) => Self::Clear,
            Some(CLAUDE_SESSION_END_RESUME_REASON) => Self::Resume,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaudeHookState {
    Running,
    WaitingForInput,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeHookCredentials {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub invocation_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub hook_token: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub helper_capability: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClaudeLifecycleEvent {
    SessionStart {
        #[serde(with = "yolo_crypto::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        source: ClaudeSessionStartSource,
        cwd: PathBuf,
        transcript_path: PathBuf,
        state: ClaudeHookState,
    },
    StateChanged {
        #[serde(with = "yolo_crypto::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        state: ClaudeHookState,
    },
    CwdChanged {
        #[serde(with = "yolo_crypto::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        cwd: PathBuf,
        transcript_path: PathBuf,
    },
    SessionEnd {
        #[serde(with = "yolo_crypto::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        reason: ClaudeSessionEndReason,
    },
    /// A synchronous permission prompt. The Worker holds the hook connection
    /// until a typed decision arrives; the lifecycle reducer rejects this
    /// variant because it never becomes durable event history.
    InteractionRequested {
        #[serde(with = "yolo_crypto::serde_uuid")]
        event_id: Uuid,
        provider_session_id: String,
        #[serde(with = "yolo_crypto::serde_uuid")]
        interaction: Uuid,
        tool_name: String,
        prompt: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum ClaudeHookOperation {
    Begin {
        provider_session_id: String,
        discover_session: bool,
        mode: ClaudeInvocationMode,
        cwd: PathBuf,
        data_home: PathBuf,
    },
    Lifecycle {
        event_sequence: u64,
        event: ClaudeLifecycleEvent,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeHookRequest {
    pub schema_version: u32,
    pub credentials: ClaudeHookCredentials,
    pub operation: ClaudeHookOperation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeProcessIdentity {
    pub pid: u32,
    pub start_time: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeCustodyAuthority {
    pub worker: ClaudeProcessIdentity,
    pub worker_boot_id: String,
    pub tty_path: PathBuf,
    pub pty_capability: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handshake_deadline_ns: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeHookResponse {
    pub schema_version: u32,
    pub accepted: bool,
    pub revision: Option<u64>,
    pub generation: Option<u64>,
    pub custody: Option<ClaudeCustodyAuthority>,
    pub code: String,
}
