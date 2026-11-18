use super::CodexOccurrenceIdentity;
use super::constants::errors;
use super::outbox::CodexOutboxState;
use super::outbox_pending_witness::CodexPendingOccurrenceWitness;
use crate::agent::hooks::HookEnvelope;
use anyhow::{Result, ensure};

pub(super) fn validate(state: &CodexOutboxState) -> Result<()> {
    for witness in &state.pending_occurrence_witnesses {
        witness.validate()?;
        let pending = state.pending.iter().find(|event| {
            event.event_id == witness.event_id() || event.sequence == witness.sequence()
        });
        let Some(pending) = pending else {
            return Err(anyhow::anyhow!(errors::OUTBOX_CORRUPT));
        };
        ensure!(
            witness.assign_envelope(pending)? == *pending,
            errors::OUTBOX_CORRUPT
        );
    }
    Ok(())
}

pub(super) fn assign_replay(
    state: &CodexOutboxState,
    incoming: &HookEnvelope,
    identity: &CodexOccurrenceIdentity,
) -> Result<Option<HookEnvelope>> {
    let witness = state
        .pending_occurrence_witnesses
        .iter()
        .find(|witness| witness.event_id() == incoming.event_id);
    witness
        .map(|witness| witness.assign(incoming, identity))
        .transpose()
}

pub(super) fn insert(
    state: &mut CodexOutboxState,
    witness: CodexPendingOccurrenceWitness,
) -> Result<()> {
    super::outbox_occurrence_state::ensure_capacity(state)?;
    super::outbox_occurrence_state::ensure_unique(state, witness.event_id(), witness.sequence())?;
    state.pending_occurrence_witnesses.push_back(witness);
    Ok(())
}

pub(super) fn take_exact(
    state: &mut CodexOutboxState,
    expected: &CodexPendingOccurrenceWitness,
) -> Result<CodexPendingOccurrenceWitness> {
    let index = state
        .pending_occurrence_witnesses
        .iter()
        .position(|witness| {
            witness.event_id() == expected.event_id() || witness.sequence() == expected.sequence()
        })
        .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
    ensure!(
        state.pending_occurrence_witnesses.get(index) == Some(expected),
        errors::OUTBOX_CORRUPT
    );
    state
        .pending_occurrence_witnesses
        .remove(index)
        .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))
}

pub(super) fn contains_event(state: &CodexOutboxState, event: &HookEnvelope) -> Result<bool> {
    let witness = state.pending_occurrence_witnesses.iter().find(|witness| {
        witness.event_id() == event.event_id || witness.sequence() == event.sequence
    });
    if let Some(witness) = witness {
        ensure!(
            witness.assign_envelope(event)? == *event,
            errors::OUTBOX_CORRUPT
        );
    }
    Ok(witness.is_some())
}

pub(super) fn proves(
    state: &CodexOutboxState,
    incoming: &HookEnvelope,
    identity: &CodexOccurrenceIdentity,
) -> Result<bool> {
    let witness = state.pending_occurrence_witnesses.iter().find(|witness| {
        witness.event_id() == incoming.event_id || witness.sequence() == incoming.sequence
    });
    let Some(witness) = witness else {
        return Ok(false);
    };
    ensure!(
        witness.assign(incoming, identity)? == *incoming,
        errors::OUTBOX_CORRUPT
    );
    Ok(true)
}
