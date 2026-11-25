use super::constants::CURRENT_SCHEMA_VERSION;
use super::event::{EventKind, HookEnvelope, SessionEndReason, SessionStartSource};
use super::identity::HookProviderKind;
use super::state::NormalizedLifecycle;
use super::violation::HookViolation;

pub(super) fn validate_envelope(
    state: &NormalizedLifecycle,
    envelope: &HookEnvelope,
) -> Result<(), HookViolation> {
    if envelope.schema_version != CURRENT_SCHEMA_VERSION || envelope.event_id.is_nil() {
        return Err(HookViolation::InvalidEnvelope);
    }
    envelope.identity.validate()?;
    let expected = &state.expected_identity;
    if envelope.identity.terminal_id != expected.terminal_id
        || envelope.identity.runtime_epoch != expected.runtime_epoch
        || envelope.identity.invocation_id != expected.invocation_id
        || envelope.identity.hook_token != expected.hook_token
    {
        return Err(HookViolation::InvalidIdentity);
    }
    if envelope.identity.generation != expected.generation {
        return Err(HookViolation::GenerationMismatch);
    }
    if envelope.identity.process != expected.process {
        return Err(HookViolation::ProcessMismatch);
    }
    if envelope.provider != state.provider {
        return Err(HookViolation::ProviderMismatch);
    }
    if envelope.provider_version != state.provider_version {
        return Err(HookViolation::VersionMismatch);
    }
    if !envelope.has_bounded_storage_encoding() {
        return Err(HookViolation::InvalidEnvelope);
    }
    if !envelope.metadata.is_valid() {
        return Err(HookViolation::InvalidEnvelope);
    }
    validate_provider_contract(envelope)
}

fn validate_provider_contract(envelope: &HookEnvelope) -> Result<(), HookViolation> {
    if envelope.provider != HookProviderKind::Codex {
        return Ok(());
    }
    match &envelope.event {
        EventKind::SessionStart {
            source:
                SessionStartSource::Startup
                | SessionStartSource::Resume
                | SessionStartSource::Clear
                | SessionStartSource::Compact,
        }
        | EventKind::SessionEnd {
            reason: SessionEndReason::Other,
        }
        | EventKind::UserPromptSubmit
        | EventKind::PreToolUse
        | EventKind::PermissionRequest
        | EventKind::PostToolUse
        | EventKind::PreCompact
        | EventKind::PostCompact
        | EventKind::SubagentStart
        | EventKind::SubagentStop
        | EventKind::Stop => Ok(()),
        _ => Err(HookViolation::UnsupportedEvent),
    }
}

pub(super) fn validate_topology(
    state: &NormalizedLifecycle,
    envelope: &HookEnvelope,
) -> Result<(), HookViolation> {
    if state.provider_session_id.is_none() {
        return match &envelope.event {
            EventKind::SessionStart {
                source:
                    SessionStartSource::Startup
                    | SessionStartSource::Resume
                    | SessionStartSource::Clear
                    | SessionStartSource::Fork,
            } if envelope.agent_id.is_none() => Ok(()),
            _ => Err(HookViolation::SessionNotStarted),
        };
    }
    if let EventKind::SessionStart { source } = &envelope.event {
        return match source {
            SessionStartSource::Compact
                if envelope.agent_id.is_none()
                    && state.provider_session_id.as_deref()
                        == Some(envelope.provider_session_id.as_str()) =>
            {
                Ok(())
            }
            SessionStartSource::Clear | SessionStartSource::Resume | SessionStartSource::Fork
                if envelope.agent_id.is_none() =>
            {
                Ok(())
            }
            SessionStartSource::Startup => Err(HookViolation::InvalidTopology),
            _ => Err(HookViolation::InvalidTopology),
        };
    }
    if state.provider_session_id.as_deref() != Some(envelope.provider_session_id.as_str()) {
        return Err(HookViolation::SessionMismatch);
    }
    match (&envelope.event, envelope.agent_id.as_deref()) {
        (EventKind::SubagentStart, Some(agent)) => {
            if state.children.contains_key(agent) {
                Err(HookViolation::InvalidTopology)
            } else {
                Ok(())
            }
        }
        (EventKind::SubagentStop, Some(agent)) => state
            .children
            .get(agent)
            .filter(|child| child.active)
            .map(|_| ())
            .ok_or(HookViolation::InvalidTopology),
        (EventKind::SubagentStart | EventKind::SubagentStop, None) => {
            Err(HookViolation::InvalidTopology)
        }
        (_, Some(agent)) if !state.children.contains_key(agent) => {
            Err(HookViolation::InvalidTopology)
        }
        (EventKind::SessionEnd { .. }, Some(_)) => Err(HookViolation::InvalidTopology),
        _ => Ok(()),
    }
}

pub(super) fn is_unknown_child_gap(state: &NormalizedLifecycle, envelope: &HookEnvelope) -> bool {
    envelope.agent_id.as_deref().is_some_and(|agent| {
        !state.children.contains_key(agent)
            && !matches!(
                &envelope.event,
                EventKind::SessionStart { .. }
                    | EventKind::SessionEnd { .. }
                    | EventKind::SubagentStart
            )
    })
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/admission_matrix_tests.rs"]
mod matrix_tests;
