use crate::agent::hooks::{
    EventSequence, FIRST_SEQUENCE, HookEnvelope, HookEventRevocationProof, HookViolation,
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

use super::constants::{MAX_DURABLE_REJECTION_BYTES, MAX_DURABLE_REJECTIONS, errors};
use super::outbox::CodexOutboxState;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexDurableRejection {
    proof: HookEventRevocationProof,
}

impl CodexDurableRejection {
    pub(crate) fn capture(event: &HookEnvelope, violation: HookViolation) -> Result<Self> {
        ensure!(violation.is_durable_rejection(), errors::OUTBOX_CORRUPT);
        let proof = HookEventRevocationProof::capture(event, violation)
            .map_err(|_| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
        let marker = Self { proof };
        marker.validate()?;
        Ok(marker)
    }

    pub fn event_id(self) -> uuid::Uuid {
        self.proof.event_id()
    }

    pub fn sequence(self) -> EventSequence {
        self.proof.sequence()
    }

    pub fn violation(self) -> HookViolation {
        self.proof.violation()
    }

    pub const fn proof(self) -> HookEventRevocationProof {
        self.proof
    }

    pub(crate) fn validate(self) -> Result<()> {
        ensure!(!self.event_id().is_nil(), errors::OUTBOX_CORRUPT);
        ensure!(
            self.sequence() != EventSequence::ZERO,
            errors::OUTBOX_CORRUPT
        );
        ensure!(
            self.violation().is_durable_rejection(),
            errors::OUTBOX_CORRUPT
        );
        let encoded =
            serde_json::to_vec(&self).map_err(|_| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
        ensure!(
            encoded.len() <= MAX_DURABLE_REJECTION_BYTES,
            errors::OUTBOX_CORRUPT
        );
        Ok(())
    }

    pub fn validate_for(self, event: &HookEnvelope) -> Result<()> {
        self.validate()?;
        ensure!(self.proof.matches(event), errors::OUTBOX_CORRUPT);
        Ok(())
    }
}

pub(super) fn find(
    state: &CodexOutboxState,
    event: &HookEnvelope,
) -> Result<Option<CodexDurableRejection>> {
    let marker = state
        .durable_rejections
        .iter()
        .find(|marker| marker.event_id() == event.event_id || marker.sequence() == event.sequence);
    if let Some(marker) = marker {
        ensure!(marker.event_id() == event.event_id, errors::OUTBOX_CORRUPT);
        ensure!(marker.sequence() == event.sequence, errors::OUTBOX_CORRUPT);
        return Ok(Some(*marker));
    }
    Ok(None)
}

pub(super) fn insert(
    state: &mut CodexOutboxState,
    event: &HookEnvelope,
    violation: HookViolation,
) -> Result<CodexDurableRejection> {
    let marker = CodexDurableRejection::capture(event, violation)?;
    ensure!(
        state.durable_rejections.len() < MAX_DURABLE_REJECTIONS,
        errors::OUTBOX_FULL
    );
    ensure!(
        state.durable_rejections.iter().all(|existing| {
            existing.event_id() != marker.event_id() && existing.sequence() != marker.sequence()
        }),
        errors::OUTBOX_CORRUPT
    );
    state.durable_rejections.push_back(marker);
    Ok(marker)
}

pub(super) fn validate(state: &CodexOutboxState) -> Result<()> {
    ensure!(
        state.durable_rejections.len() <= MAX_DURABLE_REJECTIONS,
        errors::OUTBOX_CORRUPT
    );
    let mut event_ids = BTreeSet::new();
    let mut sequences = BTreeSet::new();
    let mut previous_sequence = None;
    for marker in &state.durable_rejections {
        marker.validate()?;
        ensure!(event_ids.insert(marker.event_id()), errors::OUTBOX_CORRUPT);
        ensure!(
            sequences.insert(marker.sequence().0),
            errors::OUTBOX_CORRUPT
        );
        if let Some(previous) = previous_sequence {
            ensure!(marker.sequence().0 > previous, errors::OUTBOX_CORRUPT);
        }
        previous_sequence = Some(marker.sequence().0);
        let event = state
            .pending
            .iter()
            .chain(state.acknowledged_events.iter())
            .find(|event| {
                event.event_id == marker.event_id() || event.sequence == marker.sequence()
            });
        let Some(event) = event else {
            return Err(anyhow::anyhow!(errors::OUTBOX_CORRUPT));
        };
        ensure!(
            event.event_id == marker.event_id() && event.sequence == marker.sequence(),
            errors::OUTBOX_CORRUPT
        );
        marker.validate_for(event)?;
    }
    if let Some(marker) = state.durable_rejections.back() {
        let retained_tip = state
            .pending
            .back()
            .or_else(|| state.acknowledged_events.back())
            .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
        ensure!(
            retained_tip.event_id == marker.event_id()
                && retained_tip.sequence == marker.sequence(),
            errors::OUTBOX_CORRUPT
        );
    }
    Ok(())
}

pub(super) fn trim(state: &mut CodexOutboxState) {
    state.durable_rejections.retain(|marker| {
        state
            .pending
            .iter()
            .chain(state.acknowledged_events.iter())
            .any(|event| event.event_id == marker.event_id() && event.sequence == marker.sequence())
    });
}

pub fn validate_watermark(state: &CodexOutboxState, installed: u64) -> Result<()> {
    if state.acknowledged_sequence <= installed {
        return Ok(());
    }
    ensure!(
        state.acknowledged_sequence == installed.saturating_add(FIRST_SEQUENCE),
        errors::OUTBOX_CORRUPT
    );
    let marker = state
        .durable_rejections
        .iter()
        .find(|marker| marker.sequence().0 == state.acknowledged_sequence)
        .copied()
        .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
    let acknowledged = state
        .acknowledged_events
        .iter()
        .find(|event| event.sequence == marker.sequence())
        .ok_or_else(|| anyhow::anyhow!(errors::OUTBOX_CORRUPT))?;
    ensure!(
        acknowledged.event_id == marker.event_id(),
        errors::OUTBOX_CORRUPT
    );
    Ok(())
}

impl CodexOutboxState {
    pub fn durable_rejection_for(
        &self,
        event: &HookEnvelope,
    ) -> Result<Option<CodexDurableRejection>> {
        find(self, event)
    }
}
