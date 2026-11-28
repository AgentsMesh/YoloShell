use crate::event::{EventKind, HookEnvelope, NotificationKind};
use crate::identity::EventSequence;
use crate::snapshot::convert::{identity_from_dto, identity_to_dto, provider_from_dto};
use crate::snapshot::convert::{provider_to_dto, version_from_dto, version_to_dto};
use crate::snapshot::event::wire::{EventDto, EventEnvelopeDto, NotificationDto};

impl From<&HookEnvelope> for EventEnvelopeDto {
    fn from(envelope: &HookEnvelope) -> Self {
        Self {
            schema_version: envelope.schema_version,
            event_id: envelope.event_id,
            sequence: envelope.sequence.0,
            provider: provider_to_dto(&envelope.provider),
            provider_version: version_to_dto(envelope.provider_version),
            identity: identity_to_dto(&envelope.identity),
            provider_session_id: envelope.provider_session_id.clone(),
            turn_id: envelope.turn_id.clone(),
            agent_id: envelope.agent_id.clone(),
            tool_name: envelope.tool_name.clone(),
            tool_use_id: envelope.tool_use_id.clone(),
            stop_hook_active: envelope.stop_hook_active,
            metadata: envelope.metadata.clone(),
            event: EventDto::from(&envelope.event),
        }
    }
}

impl From<EventEnvelopeDto> for HookEnvelope {
    fn from(envelope: EventEnvelopeDto) -> Self {
        Self {
            schema_version: envelope.schema_version,
            event_id: envelope.event_id,
            sequence: EventSequence(envelope.sequence),
            provider: provider_from_dto(envelope.provider),
            provider_version: version_from_dto(envelope.provider_version),
            identity: identity_from_dto(envelope.identity),
            provider_session_id: envelope.provider_session_id,
            turn_id: envelope.turn_id,
            agent_id: envelope.agent_id,
            tool_name: envelope.tool_name,
            tool_use_id: envelope.tool_use_id,
            stop_hook_active: envelope.stop_hook_active,
            metadata: envelope.metadata,
            event: EventKind::from(envelope.event),
        }
    }
}

impl From<&EventKind> for EventDto {
    fn from(event: &EventKind) -> Self {
        match event {
            EventKind::SessionStart { source } => Self::SessionStart {
                source: source.clone(),
            },
            EventKind::SessionEnd { reason } => Self::SessionEnd {
                reason: reason.clone(),
            },
            EventKind::UserPromptSubmit => Self::UserPromptSubmit {},
            EventKind::TurnEnded => Self::TurnEnded {},
            EventKind::PreToolUse => Self::PreToolUse {},
            EventKind::PermissionRequest => Self::PermissionRequest {},
            EventKind::PostToolUse => Self::PostToolUse {},
            EventKind::PostToolUseFailure => Self::PostToolUseFailure {},
            EventKind::PreCompact => Self::PreCompact {},
            EventKind::PostCompact => Self::PostCompact {},
            EventKind::SubagentStart => Self::SubagentStart {},
            EventKind::SubagentStop => Self::SubagentStop {},
            EventKind::Stop => Self::Stop {},
            EventKind::StopFailure => Self::StopFailure {},
            EventKind::Notification { kind } => Self::Notification {
                notification: NotificationDto::from(kind),
            },
            EventKind::PermissionDenied => Self::PermissionDenied {},
            EventKind::CwdChanged => Self::CwdChanged {},
            EventKind::Elicitation => Self::Elicitation {},
            EventKind::ElicitationResult => Self::ElicitationResult {},
        }
    }
}

impl From<EventDto> for EventKind {
    fn from(event: EventDto) -> Self {
        match event {
            EventDto::SessionStart { source } => Self::SessionStart { source },
            EventDto::SessionEnd { reason } => Self::SessionEnd { reason },
            EventDto::UserPromptSubmit {} => Self::UserPromptSubmit,
            EventDto::TurnEnded {} => Self::TurnEnded,
            EventDto::PreToolUse {} => Self::PreToolUse,
            EventDto::PermissionRequest {} => Self::PermissionRequest,
            EventDto::PostToolUse {} => Self::PostToolUse,
            EventDto::PostToolUseFailure {} => Self::PostToolUseFailure,
            EventDto::PreCompact {} => Self::PreCompact,
            EventDto::PostCompact {} => Self::PostCompact,
            EventDto::SubagentStart {} => Self::SubagentStart,
            EventDto::SubagentStop {} => Self::SubagentStop,
            EventDto::Stop {} => Self::Stop,
            EventDto::StopFailure {} => Self::StopFailure,
            EventDto::Notification { notification } => Self::Notification {
                kind: NotificationKind::from(notification),
            },
            EventDto::PermissionDenied {} => Self::PermissionDenied,
            EventDto::CwdChanged {} => Self::CwdChanged,
            EventDto::Elicitation {} => Self::Elicitation,
            EventDto::ElicitationResult {} => Self::ElicitationResult,
        }
    }
}

impl From<&NotificationKind> for NotificationDto {
    fn from(kind: &NotificationKind) -> Self {
        match kind {
            NotificationKind::PermissionPrompt => Self::PermissionPrompt {},
            NotificationKind::AgentNeedsInput => Self::AgentNeedsInput {},
            NotificationKind::Elicitation => Self::Elicitation {},
            NotificationKind::Other(name) => Self::Other { name: name.clone() },
        }
    }
}

impl From<NotificationDto> for NotificationKind {
    fn from(kind: NotificationDto) -> Self {
        match kind {
            NotificationDto::PermissionPrompt {} => Self::PermissionPrompt,
            NotificationDto::AgentNeedsInput {} => Self::AgentNeedsInput,
            NotificationDto::Elicitation {} => Self::Elicitation,
            NotificationDto::Other { name } => Self::Other(name),
        }
    }
}
