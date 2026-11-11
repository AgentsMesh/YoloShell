use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::{CodexDurableRejection, CodexOutboxState};
use std::collections::VecDeque;

impl CodexOutboxState {
    pub fn acknowledged_fixture_for_test(
        owner: crate::agent::hooks::HookIdentity,
        acknowledged_sequence: u64,
        next_sequence: u64,
        events: VecDeque<HookEnvelope>,
    ) -> Self {
        Self {
            acknowledged_sequence,
            next_sequence,
            acknowledged_events: events,
            owner: Some(owner),
            ..Self::default()
        }
    }

    pub fn pending_for_test(&self) -> &VecDeque<HookEnvelope> {
        &self.pending
    }

    pub fn pending_mut_for_test(&mut self) -> &mut VecDeque<HookEnvelope> {
        &mut self.pending
    }

    pub fn acknowledged_events_for_test(&self) -> &VecDeque<HookEnvelope> {
        &self.acknowledged_events
    }

    pub fn durable_rejections_for_test(&self) -> &VecDeque<CodexDurableRejection> {
        &self.durable_rejections
    }

    pub fn durable_rejections_mut_for_test(&mut self) -> &mut VecDeque<CodexDurableRejection> {
        &mut self.durable_rejections
    }

    pub fn pending_occurrence_witness_count_for_test(&self) -> usize {
        self.pending_occurrence_witnesses.len()
    }

    pub fn occurrence_receipt_count_for_test(&self) -> usize {
        self.occurrence_receipts.len()
    }

    pub fn clear_occurrence_receipts_for_test(&mut self) {
        self.occurrence_receipts.clear();
    }
}
