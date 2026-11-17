use super::{ClaudeLifecycleEvent, ClaudeProcessIdentity};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CLAUDE_NATIVE_SCHEMA_VERSION: u32 = 3;
/// Persisted outbox and generation records written before interaction support.
pub const PREVIOUS_CLAUDE_NATIVE_SCHEMA_VERSION: u32 = 2;
pub const CLAUDE_NATIVE_OUTBOX_FILE: &str = "native-state.json";
pub use yolo_agent_runtime::{
    NATIVE_CALLBACK_SHELLS, NATIVE_HOOK_EXECUTABLE_ENV, NATIVE_HOOK_HOME_ENV,
};
pub const CLAUDE_NATIVE_MAX_ANCESTORS: usize = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeNativeScope {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub terminal_id: Uuid,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub runtime_epoch: Uuid,
    pub agent: ClaudeProcessIdentity,
}

impl ClaudeNativeScope {
    pub fn is_valid(self) -> bool {
        !self.terminal_id.is_nil()
            && !self.runtime_epoch.is_nil()
            && self.agent.pid != u32::default()
            && self.agent.start_time != u64::default()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeNativeRequest {
    pub schema_version: u32,
    pub scope: ClaudeNativeScope,
    pub event_sequence: u64,
    pub event: ClaudeLifecycleEvent,
}

impl ClaudeNativeRequest {
    pub fn is_valid(&self) -> bool {
        self.schema_version == CLAUDE_NATIVE_SCHEMA_VERSION
            && self.scope.is_valid()
            && self.event_sequence >= super::INITIAL_CLAUDE_EVENT_SEQUENCE
            && !self.event.event_id().is_nil()
            && match &self.event {
                ClaudeLifecycleEvent::InteractionRequested {
                    interaction,
                    tool_name,
                    prompt,
                    ..
                } => {
                    !interaction.is_nil()
                        && valid_text(tool_name, super::MAX_CLAUDE_PERMISSION_TOOL_NAME_BYTES)
                        && prompt.len() <= super::MAX_CLAUDE_PERMISSION_ENVELOPE_BYTES
                        && !prompt.contains('\0')
                }
                _ => true,
            }
    }
}

fn valid_text(value: &str, maximum: usize) -> bool {
    !value.is_empty() && value.len() <= maximum && !value.chars().any(char::is_control)
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeNativeResponse {
    pub schema_version: u32,
    pub accepted: bool,
    pub code: String,
    pub scope: ClaudeNativeScope,
    pub event_sequence: u64,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub event_id: Uuid,
    pub invocation_id: Option<Uuid>,
    pub revision: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interaction: Option<ClaudeInteractionDecision>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaudeInteractionDecision {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub interaction_id: Uuid,
    pub allow: bool,
}

impl ClaudeNativeResponse {
    pub fn acknowledges(&self, request: &ClaudeNativeRequest) -> bool {
        self.schema_version == CLAUDE_NATIVE_SCHEMA_VERSION
            && self.accepted
            && self.code == super::CLAUDE_HOOK_ACCEPTED
            && self.scope == request.scope
            && self.event_sequence == request.event_sequence
            && self.event_id == request.event.event_id()
            && self.invocation_id.is_some_and(|id| !id.is_nil())
            && self
                .revision
                .is_some_and(|revision| revision != u64::default())
    }
}

impl ClaudeLifecycleEvent {
    pub fn event_id(&self) -> Uuid {
        match self {
            Self::SessionStart { event_id, .. }
            | Self::StateChanged { event_id, .. }
            | Self::CwdChanged { event_id, .. }
            | Self::SessionEnd { event_id, .. }
            | Self::InteractionRequested { event_id, .. } => *event_id,
        }
    }

    pub fn provider_session_id(&self) -> &str {
        match self {
            Self::SessionStart {
                provider_session_id, ..
            }
            | Self::StateChanged {
                provider_session_id, ..
            }
            | Self::CwdChanged {
                provider_session_id, ..
            }
            | Self::SessionEnd {
                provider_session_id, ..
            }
            | Self::InteractionRequested {
                provider_session_id, ..
            } => provider_session_id,
        }
    }
}
