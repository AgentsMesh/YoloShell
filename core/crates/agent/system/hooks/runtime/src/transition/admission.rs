use crate::event::{EventKind, HookEnvelope, SessionStartSource};
use crate::state::{CompactionPhase, NormalizedLifecycle, TerminationCandidate};
use crate::violation::HookViolation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EventAdmission {
    Apply,
    Stale,
}

pub(crate) fn admit_terminal_event(
    state: &NormalizedLifecycle,
    envelope: &HookEnvelope,
) -> Result<Option<EventAdmission>, HookViolation> {
    let Some(termination) = state.termination_candidate else {
        return Ok(None);
    };
    match &envelope.event {
        EventKind::SessionStart { source } if accepts_start(termination, source) => Ok(None),
        EventKind::SessionStart { .. } => Err(HookViolation::InvalidTopology),
        _ => {
            validate_terminal_shape(state, envelope)?;
            Ok(Some(EventAdmission::Stale))
        }
    }
}

pub(crate) fn admit_active_event(
    state: &NormalizedLifecycle,
    envelope: &HookEnvelope,
) -> EventAdmission {
    if targets_inactive_child(state, envelope) {
        return EventAdmission::Stale;
    }
    match &envelope.event {
        EventKind::PreCompact if compaction(state, envelope).is_some() => EventAdmission::Stale,
        EventKind::PostCompact => admit_completion(compaction(state, envelope), envelope),
        EventKind::SessionStart {
            source: SessionStartSource::Compact,
        } => admit_completion(state.compaction.as_ref(), envelope),
        _ => EventAdmission::Apply,
    }
}

fn targets_inactive_child(state: &NormalizedLifecycle, envelope: &HookEnvelope) -> bool {
    envelope
        .agent_id
        .as_deref()
        .is_some_and(|agent| state.children.get(agent).is_some_and(|child| !child.active))
}

fn accepts_start(termination: TerminationCandidate, source: &SessionStartSource) -> bool {
    matches!(
        (termination, source),
        (TerminationCandidate::Clear, SessionStartSource::Clear)
            | (TerminationCandidate::Resume, SessionStartSource::Resume)
    )
}

fn validate_terminal_shape(
    state: &NormalizedLifecycle,
    envelope: &HookEnvelope,
) -> Result<(), HookViolation> {
    if state.provider_session_id.as_deref() != Some(envelope.provider_session_id.as_str()) {
        return Err(HookViolation::SessionMismatch);
    }
    match (&envelope.event, envelope.agent_id.as_deref()) {
        (EventKind::SubagentStart | EventKind::SubagentStop, None)
        | (EventKind::SessionEnd { .. }, Some(_)) => Err(HookViolation::InvalidTopology),
        _ => Ok(()),
    }
}

fn compaction<'a>(
    state: &'a NormalizedLifecycle,
    envelope: &HookEnvelope,
) -> Option<&'a CompactionPhase> {
    envelope.agent_id.as_deref().map_or_else(
        || state.compaction.as_ref(),
        |agent| state.child_compactions.get(agent),
    )
}

fn admit_completion(
    compaction: Option<&CompactionPhase>,
    envelope: &HookEnvelope,
) -> EventAdmission {
    if compaction.is_some_and(|phase| phase.admits_completion(envelope.sequence)) {
        EventAdmission::Apply
    } else {
        EventAdmission::Stale
    }
}
