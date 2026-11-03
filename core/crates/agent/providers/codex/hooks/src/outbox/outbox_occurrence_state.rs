use super::CodexOccurrenceIdentity;
use super::constants::{MAX_OCCURRENCE_BINDINGS, errors};
use super::outbox::CodexOutboxState;
use crate::agent::hooks::{EventSequence, HookEnvelope};
use anyhow::{Result, ensure};
use std::collections::BTreeSet;

pub(super) fn validate(state: &CodexOutboxState) -> Result<()> {
    ensure!(
        binding_count(state)? <= MAX_OCCURRENCE_BINDINGS,
        errors::OUTBOX_CORRUPT
    );
    let mut event_ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    for (event_id, sequence) in bindings(state) {
        ensure!(
            event_ids.insert(event_id) && sequences.insert(sequence),
            errors::OUTBOX_CORRUPT
        );
    }
    super::outbox_pending_witness_state::validate(state)?;
    super::outbox_receipt_state::validate(state)
}

pub(super) fn find_replay(
    state: &CodexOutboxState,
    incoming: &HookEnvelope,
    identity: Option<&CodexOccurrenceIdentity>,
) -> Result<Option<HookEnvelope>> {
    if let Some(event) = state
        .pending
        .iter()
        .find(|event| event.event_id == incoming.event_id)
    {
        ensure_same_event(event, incoming)?;
        return match identity {
            Some(identity) => {
                super::outbox_pending_witness_state::assign_replay(state, incoming, identity)?
                    .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))
                    .map(Some)
            }
            None => {
                ensure!(
                    !super::outbox_pending_witness_state::contains_event(state, event)?,
                    errors::OUTBOX_CORRUPT
                );
                Ok(Some(event.clone()))
            }
        };
    }
    if let Some(event) = state
        .acknowledged_events
        .iter()
        .find(|event| event.event_id == incoming.event_id)
    {
        ensure_same_event(event, incoming)?;
        return match identity {
            Some(identity) => {
                super::outbox_receipt_state::assign_replay(state, incoming, identity)?
                    .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))
                    .map(Some)
            }
            None => {
                ensure!(
                    !state.occurrence_receipts.iter().any(|receipt| {
                        receipt.event_id() == event.event_id || receipt.sequence() == event.sequence
                    }),
                    errors::OUTBOX_CORRUPT
                );
                Ok(Some(event.clone()))
            }
        };
    }
    match identity {
        Some(identity) => super::outbox_receipt_state::assign_replay(state, incoming, identity),
        None => {
            ensure!(
                !state
                    .occurrence_receipts
                    .iter()
                    .any(|receipt| receipt.event_id() == incoming.event_id),
                errors::OUTBOX_CORRUPT
            );
            Ok(None)
        }
    }
}

pub(super) fn ensure_capacity(state: &CodexOutboxState) -> Result<()> {
    ensure!(
        binding_count(state)? < MAX_OCCURRENCE_BINDINGS,
        errors::OUTBOX_FULL
    );
    Ok(())
}

pub(super) fn has_capacity(state: &CodexOutboxState) -> Result<bool> {
    Ok(binding_count(state)? < MAX_OCCURRENCE_BINDINGS)
}

impl CodexOutboxState {
    pub fn has_occurrence_binding(&self, incoming: &HookEnvelope) -> bool {
        self.pending_occurrence_witnesses.iter().any(|witness| {
            witness.event_id() == incoming.event_id || witness.sequence() == incoming.sequence
        }) || self.occurrence_receipts.iter().any(|receipt| {
            receipt.event_id() == incoming.event_id || receipt.sequence() == incoming.sequence
        })
    }
}

pub(super) fn ensure_unique(
    state: &CodexOutboxState,
    event_id: uuid::Uuid,
    sequence: EventSequence,
) -> Result<()> {
    ensure!(
        bindings(state).all(|(existing_id, existing_sequence)| {
            existing_id != event_id && existing_sequence != sequence
        }),
        errors::OUTBOX_CORRUPT
    );
    Ok(())
}

fn binding_count(state: &CodexOutboxState) -> Result<usize> {
    state
        .pending_occurrence_witnesses
        .len()
        .checked_add(state.occurrence_receipts.len())
        .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))
}

fn bindings(state: &CodexOutboxState) -> impl Iterator<Item = (uuid::Uuid, EventSequence)> + '_ {
    state
        .pending_occurrence_witnesses
        .iter()
        .map(|witness| (witness.event_id(), witness.sequence()))
        .chain(
            state
                .occurrence_receipts
                .iter()
                .map(|receipt| (receipt.event_id(), receipt.sequence())),
        )
}

fn ensure_same_event(retained: &HookEnvelope, incoming: &HookEnvelope) -> Result<()> {
    ensure!(
        super::outbox_validation::same_event(retained, incoming),
        errors::OUTBOX_CORRUPT
    );
    Ok(())
}
