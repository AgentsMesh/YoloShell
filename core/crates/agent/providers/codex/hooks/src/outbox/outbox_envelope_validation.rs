use super::constants::{ACCEPTED_PROVIDER_VERSION, errors};
use crate::agent::hooks::{
    EventKind, HookEnvelope, HookViolation, HookProviderKind, SessionEndReason, SessionStartSource,
};
use anyhow::{Result, ensure};

pub(super) fn validate(event: &HookEnvelope) -> Result<()> {
    validate_common(event)?;
    ensure!(
        event.provider == HookProviderKind::Codex,
        errors::OUTBOX_CORRUPT
    );
    ensure!(
        event.provider_version == ACCEPTED_PROVIDER_VERSION,
        errors::OUTBOX_CORRUPT
    );
    validate_shape(event)
}

pub(super) fn validate_rejected(event: &HookEnvelope, violation: HookViolation) -> Result<()> {
    ensure!(violation.is_durable_rejection(), errors::OUTBOX_CORRUPT);
    validate_common(event)?;
    match violation {
        HookViolation::ProviderMismatch => ensure!(
            event.provider != HookProviderKind::Codex,
            errors::OUTBOX_CORRUPT
        ),
        HookViolation::VersionMismatch => ensure!(
            event.provider == HookProviderKind::Codex
                && event.provider_version != ACCEPTED_PROVIDER_VERSION,
            errors::OUTBOX_CORRUPT
        ),
        HookViolation::UnsupportedEvent => {
            ensure!(
                event.provider == HookProviderKind::Codex
                    && event.provider_version == ACCEPTED_PROVIDER_VERSION
                    && is_durable_unsupported_event(event),
                errors::OUTBOX_CORRUPT
            );
        }
        HookViolation::InvalidTopology => {
            ensure!(
                event.provider == HookProviderKind::Codex
                    && event.provider_version == ACCEPTED_PROVIDER_VERSION,
                errors::OUTBOX_CORRUPT
            );
            if matches!(&event.event, EventKind::SubagentStart) {
                ensure!(
                    matches!(
                        EventShape::from(event),
                        EventShape::TURN | EventShape::AGENT
                    ),
                    errors::OUTBOX_CORRUPT
                );
            } else {
                validate_shape(event)?;
            }
        }
        HookViolation::SessionMismatch => {
            ensure!(
                event.provider == HookProviderKind::Codex
                    && event.provider_version == ACCEPTED_PROVIDER_VERSION,
                errors::OUTBOX_CORRUPT
            );
            validate_shape(event)?;
        }
        _ => validate(event)?,
    }
    Ok(())
}

fn validate_common(event: &HookEnvelope) -> Result<()> {
    ensure!(
        event.schema_version == crate::agent::hooks::CURRENT_SCHEMA_VERSION
            && !event.event_id.is_nil(),
        errors::OUTBOX_CORRUPT
    );
    event
        .identity
        .validate()
        .map_err(|_| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
    ensure!(event.has_bounded_storage_encoding(), errors::OUTBOX_CORRUPT);
    ensure!(event.metadata.is_valid(), errors::OUTBOX_CORRUPT);
    Ok(())
}

fn is_durable_unsupported_event(event: &HookEnvelope) -> bool {
    matches!(
        &event.event,
        EventKind::Notification { .. } | EventKind::PostToolUseFailure
    )
}

fn validate_shape(event: &HookEnvelope) -> Result<()> {
    let shape = EventShape::from(event);
    let valid = match &event.event {
        EventKind::SessionStart {
            source:
                SessionStartSource::Startup
                | SessionStartSource::Resume
                | SessionStartSource::Clear
                | SessionStartSource::Compact,
        }
        | EventKind::SessionEnd {
            reason: SessionEndReason::Other,
        } => shape == EventShape::EMPTY,
        EventKind::UserPromptSubmit | EventKind::PreCompact | EventKind::PostCompact => {
            matches!(shape, EventShape::TURN | EventShape::AGENT)
        }
        EventKind::PreToolUse | EventKind::PostToolUse => {
            matches!(shape, EventShape::TOOL_USE | EventShape::AGENT_TOOL_USE)
        }
        EventKind::PermissionRequest => {
            matches!(shape, EventShape::TOOL | EventShape::AGENT_TOOL)
        }
        EventKind::SubagentStart => shape == EventShape::AGENT,
        EventKind::SubagentStop => shape == EventShape::AGENT_STOP,
        EventKind::Stop => shape == EventShape::STOP,
        _ => false,
    };
    ensure!(valid, errors::OUTBOX_CORRUPT);
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EventShape {
    turn: bool,
    agent: bool,
    tool: bool,
    tool_use: bool,
    stop: bool,
}

impl EventShape {
    const EMPTY: Self = Self::new(false, false, false, false, false);
    const TURN: Self = Self::new(true, false, false, false, false);
    const TOOL: Self = Self::new(true, false, true, false, false);
    const TOOL_USE: Self = Self::new(true, false, true, true, false);
    const AGENT: Self = Self::new(true, true, false, false, false);
    const AGENT_TOOL: Self = Self::new(true, true, true, false, false);
    const AGENT_TOOL_USE: Self = Self::new(true, true, true, true, false);
    const AGENT_STOP: Self = Self::new(true, true, false, false, true);
    const STOP: Self = Self::new(true, false, false, false, true);

    const fn new(turn: bool, agent: bool, tool: bool, tool_use: bool, stop: bool) -> Self {
        Self {
            turn,
            agent,
            tool,
            tool_use,
            stop,
        }
    }
}

impl From<&HookEnvelope> for EventShape {
    fn from(event: &HookEnvelope) -> Self {
        Self::new(
            event.turn_id.is_some(),
            event.agent_id.is_some(),
            event.tool_name.is_some(),
            event.tool_use_id.is_some(),
            event.stop_hook_active.is_some(),
        )
    }
}
