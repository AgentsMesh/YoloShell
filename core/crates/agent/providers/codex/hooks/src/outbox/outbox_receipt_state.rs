use super::CodexOccurrenceIdentity;
use super::constants::errors;
use super::outbox::CodexOutboxState;
use super::outbox_receipt::CodexOccurrenceReceipt;
use crate::agent::hooks::HookEnvelope;
use anyhow::{Result, ensure};

pub(super) fn validate(state: &CodexOutboxState) -> Result<()> {
    for receipt in &state.occurrence_receipts {
        receipt.validate()?;
        ensure!(
            receipt.sequence().0 <= state.acknowledged_sequence,
            errors::OUTBOX_CORRUPT
        );
        ensure!(
            !state.pending.iter().any(|event| {
                event.event_id == receipt.event_id() || event.sequence == receipt.sequence()
            }),
            errors::OUTBOX_CORRUPT
        );
        if let Some(event) = state.acknowledged_events.iter().find(|event| {
            event.event_id == receipt.event_id() || event.sequence == receipt.sequence()
        }) {
            ensure!(
                receipt.assign_envelope(event)? == *event,
                errors::OUTBOX_CORRUPT
            );
        }
    }
    Ok(())
}

pub(super) fn assign_replay(
    state: &CodexOutboxState,
    incoming: &HookEnvelope,
    identity: &CodexOccurrenceIdentity,
) -> Result<Option<HookEnvelope>> {
    let receipt = state
        .occurrence_receipts
        .iter()
        .find(|receipt| receipt.event_id() == incoming.event_id);
    receipt
        .map(|receipt| receipt.assign(incoming, identity))
        .transpose()
}

pub(super) fn contains_exact(
    state: &CodexOutboxState,
    expected: &CodexOccurrenceReceipt,
) -> Result<bool> {
    let existing = state.occurrence_receipts.iter().find(|receipt| {
        receipt.event_id() == expected.event_id() || receipt.sequence() == expected.sequence()
    });
    if let Some(existing) = existing {
        ensure!(existing == expected, errors::OUTBOX_CORRUPT);
    }
    Ok(existing.is_some())
}

pub(super) fn insert(state: &mut CodexOutboxState, receipt: CodexOccurrenceReceipt) -> Result<()> {
    super::outbox_occurrence_state::ensure_capacity(state)?;
    super::outbox_occurrence_state::ensure_unique(state, receipt.event_id(), receipt.sequence())?;
    state.occurrence_receipts.push_back(receipt);
    Ok(())
}

impl CodexOutboxState {
    pub fn proves_occurrence_receipt(
        &self,
        incoming: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<bool> {
        let receipt = self.occurrence_receipts.iter().find(|receipt| {
            receipt.event_id() == incoming.event_id || receipt.sequence() == incoming.sequence
        });
        let Some(receipt) = receipt else {
            return Ok(false);
        };
        ensure!(
            receipt.assign(incoming, identity)? == *incoming,
            errors::OUTBOX_CORRUPT
        );
        Ok(true)
    }

    pub fn proves_pending_occurrence_witness(
        &self,
        incoming: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<bool> {
        super::outbox_pending_witness_state::proves(self, incoming, identity)
    }
}
