use super::constants::ROOT_PATH_COMPONENT_OFFSET;
use super::identity::{EventSequence, HookIdentity, HookProviderKind, ProviderVersion};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStartSource {
    Startup,
    Resume,
    Clear,
    Compact,
    Fork,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEndReason {
    Other,
    Clear,
    Resume,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    PermissionPrompt,
    AgentNeedsInput,
    Elicitation,
    Other(String),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum EventKind {
    SessionStart { source: SessionStartSource },
    SessionEnd { reason: SessionEndReason },
    UserPromptSubmit,
    TurnEnded,
    PreToolUse,
    PermissionRequest,
    PostToolUse,
    PostToolUseFailure,
    PreCompact,
    PostCompact,
    SubagentStart,
    SubagentStop,
    Stop,
    StopFailure,
    Notification { kind: NotificationKind },
    PermissionDenied,
    CwdChanged,
    Elicitation,
    ElicitationResult,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookMetadata {
    pub cwd: Option<PathBuf>,
    pub transcript_path: Option<PathBuf>,
}

impl HookMetadata {
    pub fn is_valid(&self) -> bool {
        [self.cwd.as_deref(), self.transcript_path.as_deref()]
            .into_iter()
            .flatten()
            .all(|path| {
                path.as_os_str().as_encoded_bytes().len()
                    <= super::constants::MAX_HOOK_METADATA_PATH_BYTES
                    && is_lexically_normalized_absolute(path)
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HookEnvelope {
    pub schema_version: u16,
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub event_id: Uuid,
    pub sequence: EventSequence,
    pub provider: HookProviderKind,
    pub provider_version: ProviderVersion,
    pub identity: HookIdentity,
    /// Opaque identifier owned by the provider's history/resume system.
    pub provider_session_id: String,
    pub turn_id: Option<String>,
    pub agent_id: Option<String>,
    pub tool_name: Option<String>,
    pub tool_use_id: Option<String>,
    pub stop_hook_active: Option<bool>,
    pub metadata: HookMetadata,
    pub event: EventKind,
}

impl HookEnvelope {
    pub fn has_bounded_storage_encoding(&self) -> bool {
        let identifiers = [
            Some(self.provider_session_id.as_str()),
            self.turn_id.as_deref(),
            self.agent_id.as_deref(),
            self.tool_name.as_deref(),
            self.tool_use_id.as_deref(),
        ];
        if identifiers.into_iter().flatten().any(|value| {
            value.is_empty() || value.len() > super::constants::MAX_HOOK_IDENTIFIER_BYTES
        }) {
            return false;
        }
        let mut maximum_sequence = self.clone();
        maximum_sequence.sequence = EventSequence(u64::MAX);
        serde_json::to_vec(&maximum_sequence)
            .is_ok_and(|encoded| encoded.len() <= super::constants::MAX_HOOK_ENVELOPE_BYTES)
    }
}

// These aliases keep event ownership separate from the reducer implementation.
pub type EventHistory = BTreeMap<EventSequence, HookEnvelope>;
pub type EventIds = BTreeMap<Uuid, EventSequence>;

fn is_lexically_normalized_absolute(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path
            .components()
            .skip(ROOT_PATH_COMPONENT_OFFSET)
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}
