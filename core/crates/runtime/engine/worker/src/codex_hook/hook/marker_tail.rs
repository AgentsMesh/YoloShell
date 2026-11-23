use super::constants::{CODEX_OUTBOX_HISTORY_ERROR, CODEX_OUTBOX_OWNER_MISSING_ERROR};
use crate::agent::hooks::{
    FIRST_SEQUENCE, HookEnvelope, HookIdentity, Reducer, RevocationEvidence,
};
use crate::codex_hooks::{CodexDurableRejection, CodexOutboxState};
use anyhow::{Context, Result, ensure};

struct RejectionTail<'a> {
    marker: CodexDurableRejection,
    event: &'a HookEnvelope,
    acknowledged: bool,
}

pub(super) fn validate(
    state: &CodexOutboxState,
    reducer: &Reducer,
    installed: u64,
    tip: u64,
) -> Result<()> {
    let marker = rejection_tail(state)?;
    if let Some(tail) = marker.as_ref() {
        ensure!(
            tail.marker.sequence().0 > installed,
            CODEX_OUTBOX_HISTORY_ERROR
        );
        ensure!(tail.marker.sequence().0 == tip, CODEX_OUTBOX_HISTORY_ERROR);
    }
    match reducer.revocation_evidence() {
        None => {
            ensure!(!reducer.is_revoked(), CODEX_OUTBOX_HISTORY_ERROR);
            if let Some(tail) = marker {
                ensure!(!tail.acknowledged, CODEX_OUTBOX_HISTORY_ERROR);
            }
        }
        Some(RevocationEvidence::HookEvent { event, proof, .. }) => {
            ensure!(reducer.is_revoked(), CODEX_OUTBOX_HISTORY_ERROR);
            let tail = marker.context(CODEX_OUTBOX_HISTORY_ERROR)?;
            ensure!(tail.event == event, CODEX_OUTBOX_HISTORY_ERROR);
            ensure!(tail.marker.proof() == *proof, CODEX_OUTBOX_HISTORY_ERROR);
            let expected = installed
                .checked_add(FIRST_SEQUENCE)
                .context(CODEX_OUTBOX_HISTORY_ERROR)?;
            ensure!(
                tail.marker.sequence().0 == expected,
                CODEX_OUTBOX_HISTORY_ERROR
            );
        }
        Some(RevocationEvidence::ProcessObservation { .. }) => {
            ensure!(reducer.is_revoked(), CODEX_OUTBOX_HISTORY_ERROR);
            ensure!(marker.is_none(), CODEX_OUTBOX_HISTORY_ERROR);
            ensure!(tip == installed, CODEX_OUTBOX_HISTORY_ERROR);
        }
    }
    Ok(())
}

fn rejection_tail(state: &CodexOutboxState) -> Result<Option<RejectionTail<'_>>> {
    let mut marker = None;
    for found in state.durable_rejections() {
        ensure!(marker.is_none(), CODEX_OUTBOX_HISTORY_ERROR);
        let event = retained_events(state)
            .find(|event| event.event_id == found.event_id() || event.sequence == found.sequence())
            .context(CODEX_OUTBOX_HISTORY_ERROR)?;
        ensure!(
            found.event_id() == event.event_id,
            CODEX_OUTBOX_HISTORY_ERROR
        );
        ensure!(
            found.sequence() == event.sequence,
            CODEX_OUTBOX_HISTORY_ERROR
        );
        found
            .validate_for(event)
            .context(CODEX_OUTBOX_HISTORY_ERROR)?;
        let acknowledged = state.is_acknowledged(event);
        marker = Some(RejectionTail {
            marker: *found,
            event,
            acknowledged,
        });
    }
    Ok(marker)
}

pub(super) fn validate_owner(
    state: &CodexOutboxState,
    owner: &HookIdentity,
    required: bool,
) -> Result<()> {
    match state.owner() {
        Some(existing) => ensure!(existing == owner, CODEX_OUTBOX_OWNER_MISSING_ERROR),
        None => ensure!(
            !required && state.next_sequence() == FIRST_SEQUENCE,
            CODEX_OUTBOX_OWNER_MISSING_ERROR
        ),
    }
    Ok(())
}

pub(super) fn validate_overlap(
    state: &CodexOutboxState,
    reducer: &Reducer,
    installed: u64,
) -> Result<()> {
    for event in retained_events(state).filter(|event| event.sequence.0 <= installed) {
        let snapshot_event = reducer.state().history.get(&event.sequence);
        if event.sequence.0 > state.acknowledged_sequence() {
            ensure!(snapshot_event == Some(event), CODEX_OUTBOX_HISTORY_ERROR);
        } else if let Some(snapshot_event) = snapshot_event {
            ensure!(snapshot_event == event, CODEX_OUTBOX_HISTORY_ERROR);
        }
    }
    Ok(())
}

pub(super) fn exact_retained(
    state: &CodexOutboxState,
    incoming: &HookEnvelope,
) -> Result<Option<bool>> {
    let Some(retained) = retained_events(state)
        .find(|event| event.event_id == incoming.event_id || event.sequence == incoming.sequence)
    else {
        return Ok(None);
    };
    ensure!(retained == incoming, CODEX_OUTBOX_HISTORY_ERROR);
    Ok(Some(state.is_acknowledged(retained)))
}

pub(super) fn retained_events(state: &CodexOutboxState) -> impl Iterator<Item = &HookEnvelope> {
    state.retained_events()
}
