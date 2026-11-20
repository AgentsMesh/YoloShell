use super::constants::{errors, values};
#[cfg(test)]
use super::identity::CodexHookIdentity;
use super::wire::CodexHookInput;
use crate::agent::hooks::{
    CURRENT_SCHEMA_VERSION, EventKind, HookEnvelope, HookMetadata, SessionEndReason,
    SessionStartSource,
};
use anyhow::{Result, ensure};
use uuid::Uuid;

#[cfg(test)]
pub(super) fn map(input: &CodexHookInput, identity: &CodexHookIdentity) -> Result<HookEnvelope> {
    map_identity(input, &identity.hook, identity.provider_version)
}

fn map_identity(
    input: &CodexHookInput,
    identity: &crate::agent::hooks::HookIdentity,
    provider_version: crate::agent::hooks::ProviderVersion,
) -> Result<HookEnvelope> {
    input.validate()?;
    let event = map_event(input)?;
    let common = input.common();
    let envelope = HookEnvelope {
        schema_version: CURRENT_SCHEMA_VERSION,
        event_id: Uuid::nil(),
        sequence: crate::agent::hooks::EventSequence::ZERO,
        provider: crate::agent::hooks::HookProviderKind::Codex,
        provider_version,
        identity: identity.clone(),
        provider_session_id: common.session_id.to_owned(),
        turn_id: input.turn_id().map(str::to_owned),
        agent_id: input.agent_id().map(str::to_owned),
        tool_name: input.tool_name().map(str::to_owned),
        tool_use_id: input.tool_use_id().map(str::to_owned),
        stop_hook_active: input.stop_hook_active(),
        metadata: HookMetadata {
            cwd: Some(common.cwd.to_path_buf()),
            transcript_path: common.transcript_path.map(std::path::Path::to_path_buf),
        },
        event,
    };
    ensure!(
        envelope.has_bounded_storage_encoding(),
        errors::INPUT_TOO_LARGE
    );
    Ok(envelope)
}

pub(super) fn map_native(
    input: &CodexHookInput,
    identity: &crate::agent::hooks::HookIdentity,
    event_id: Uuid,
) -> Result<HookEnvelope> {
    let mut envelope = map_identity(input, identity, super::ACCEPTED_PROVIDER_VERSION)?;
    envelope.event_id = event_id;
    super::outbox_envelope_validation::validate(&envelope)?;
    Ok(envelope)
}

fn map_event(input: &CodexHookInput) -> Result<EventKind> {
    Ok(match input {
        CodexHookInput::SessionStart { source, .. } => EventKind::SessionStart {
            source: start_source(source)?,
        },
        CodexHookInput::SessionEnd { reason, .. } => EventKind::SessionEnd {
            reason: end_reason(reason)?,
        },
        CodexHookInput::UserPromptSubmit { .. } => EventKind::UserPromptSubmit,
        CodexHookInput::PreToolUse { .. } => EventKind::PreToolUse,
        CodexHookInput::PermissionRequest { .. } => EventKind::PermissionRequest,
        CodexHookInput::PostToolUse { .. } => EventKind::PostToolUse,
        CodexHookInput::PreCompact { .. } => EventKind::PreCompact,
        CodexHookInput::PostCompact { .. } => EventKind::PostCompact,
        CodexHookInput::SubagentStart { .. } => EventKind::SubagentStart,
        CodexHookInput::SubagentStop { .. } => EventKind::SubagentStop,
        CodexHookInput::Stop { .. } => EventKind::Stop,
    })
}

fn start_source(value: &str) -> Result<SessionStartSource> {
    Ok(match value {
        values::STARTUP => SessionStartSource::Startup,
        values::RESUME => SessionStartSource::Resume,
        values::CLEAR => SessionStartSource::Clear,
        values::COMPACT => SessionStartSource::Compact,
        _ => return Err(anyhow::anyhow!(errors::INPUT_INVALID)),
    })
}

fn end_reason(value: &str) -> Result<SessionEndReason> {
    ensure!(value == values::OTHER, errors::INPUT_INVALID);
    Ok(SessionEndReason::Other)
}
