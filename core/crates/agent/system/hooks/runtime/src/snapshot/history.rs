use crate::admission::validate_envelope;
use crate::constants::MAX_REPLAY_WINDOW;
use crate::event::{EventHistory, EventIds, EventKind, HookEnvelope, SessionStartSource};
use crate::identity::EventSequence;
use crate::snapshot::SnapshotViolation;
use crate::snapshot::event::wire::EventEnvelopeDto;
use crate::snapshot::wire::HistoryDto;
use crate::state::NormalizedLifecycle;

pub(crate) fn history_to_dto(state: &NormalizedLifecycle) -> HistoryDto {
    HistoryDto {
        entries: state.history.values().map(EventEnvelopeDto::from).collect(),
    }
}

pub(crate) fn history_from_dto(
    state: &NormalizedLifecycle,
    dto: HistoryDto,
) -> Result<(EventHistory, EventIds), SnapshotViolation> {
    let count = dto.entries.len();
    if count > MAX_REPLAY_WINDOW {
        return Err(SnapshotViolation::HistoryTooLarge);
    }
    let retained = usize::try_from(state.last_sequence.0)
        .unwrap_or(MAX_REPLAY_WINDOW)
        .min(MAX_REPLAY_WINDOW);
    if count != retained {
        return Err(SnapshotViolation::InvalidHistory);
    }
    if count == 0 {
        return if state.last_sequence == EventSequence::ZERO {
            Ok((EventHistory::new(), EventIds::new()))
        } else {
            Err(SnapshotViolation::InvalidHistory)
        };
    }
    let span = u64::try_from(count - 1).map_err(|_| SnapshotViolation::InvalidHistory)?;
    let first = state
        .last_sequence
        .0
        .checked_sub(span)
        .filter(|sequence| *sequence != EventSequence::ZERO.0)
        .ok_or(SnapshotViolation::InvalidHistory)?;
    let mut history = EventHistory::new();
    let mut event_ids = EventIds::new();
    for (offset, dto) in dto.entries.into_iter().enumerate() {
        let offset = u64::try_from(offset).map_err(|_| SnapshotViolation::InvalidHistory)?;
        let expected = first
            .checked_add(offset)
            .ok_or(SnapshotViolation::InvalidHistory)?;
        let envelope = HookEnvelope::from(dto);
        if envelope.sequence != EventSequence(expected)
            || validate_envelope(state, &envelope).is_err()
            || !valid_static_topology(&envelope)
            || history
                .insert(envelope.sequence, envelope.clone())
                .is_some()
            || event_ids
                .insert(envelope.event_id, envelope.sequence)
                .is_some()
        {
            return Err(SnapshotViolation::InvalidHistory);
        }
    }
    if history.keys().next_back().copied() != Some(state.last_sequence) {
        return Err(SnapshotViolation::InvalidHistory);
    }
    Ok((history, event_ids))
}

fn valid_static_topology(envelope: &HookEnvelope) -> bool {
    match (&envelope.event, envelope.agent_id.as_deref()) {
        (
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
            None,
        ) => envelope.sequence == EventSequence::FIRST,
        (EventKind::SessionStart { .. } | EventKind::SessionEnd { .. }, None)
        | (EventKind::SubagentStart | EventKind::SubagentStop, Some(_)) => true,
        (EventKind::SessionStart { .. } | EventKind::SessionEnd { .. }, Some(_))
        | (EventKind::SubagentStart | EventKind::SubagentStop, None) => false,
        _ => true,
    }
}
