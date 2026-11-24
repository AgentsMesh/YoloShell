use crate::event::{HookMetadata, SessionEndReason, SessionStartSource};
use crate::snapshot::wire::{IdentityDto, ProviderDto, VersionDto};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EventEnvelopeDto {
    pub(crate) schema_version: u16,
    pub(crate) event_id: Uuid,
    pub(crate) sequence: u64,
    pub(crate) provider: ProviderDto,
    pub(crate) provider_version: VersionDto,
    pub(crate) identity: IdentityDto,
    pub(crate) provider_session_id: String,
    pub(crate) turn_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) tool_name: Option<String>,
    pub(crate) tool_use_id: Option<String>,
    pub(crate) stop_hook_active: Option<bool>,
    pub(crate) metadata: HookMetadata,
    pub(crate) event: EventDto,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum EventDto {
    SessionStart { source: SessionStartSource },
    SessionEnd { reason: SessionEndReason },
    UserPromptSubmit {},
    TurnEnded {},
    PreToolUse {},
    PermissionRequest {},
    PostToolUse {},
    PostToolUseFailure {},
    PreCompact {},
    PostCompact {},
    SubagentStart {},
    SubagentStop {},
    Stop {},
    StopFailure {},
    Notification { notification: NotificationDto },
    PermissionDenied {},
    CwdChanged {},
    Elicitation {},
    ElicitationResult {},
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum NotificationDto {
    PermissionPrompt {},
    AgentNeedsInput {},
    Elicitation {},
    Other { name: String },
}
