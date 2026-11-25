use super::constants::{FIRST_SEQUENCE, errors};
use super::lock::OutboxLock;
use super::outbox::CodexOutbox;
use super::outbox_pending_witness::CodexPendingOccurrenceWitness;
use super::outbox_receipt::CodexOccurrenceReceipt;
use super::{outbox_storage, outbox_validation};
use crate::agent::hooks::HookEnvelope;
use crate::codex_hooks::CodexOccurrenceIdentity;
use anyhow::{Context, Result, ensure};

impl CodexOutbox {
    pub fn acknowledge(&self, sequence: u64, event_id: uuid::Uuid) -> Result<()> {
        self.acknowledge_inner(sequence, event_id, None)
    }

    pub fn acknowledge_occurrence(
        &self,
        assigned: &HookEnvelope,
        occurrence: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<()> {
        let witness = CodexPendingOccurrenceWitness::capture(assigned, occurrence, identity)?;
        self.acknowledge_inner(assigned.sequence.0, assigned.event_id, Some(witness))
    }

    fn acknowledge_inner(
        &self,
        sequence: u64,
        event_id: uuid::Uuid,
        expected_witness: Option<CodexPendingOccurrenceWitness>,
    ) -> Result<()> {
        let lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let mut loaded = outbox_storage::load(&lock, &self.path)?;
        outbox_validation::check_configured(&loaded.state, self.owner.as_ref())?;
        if sequence <= loaded.state.acknowledged_sequence {
            let retained = outbox_validation::find_acknowledged(&loaded.state, sequence, event_id)?;
            let Some(witness) = expected_witness else {
                ensure!(retained, errors::ACK_REPLAY_UNKNOWN);
                return Ok(());
            };
            let receipt = CodexOccurrenceReceipt::from_binding(witness.into_binding())?;
            ensure!(
                super::outbox_receipt_state::contains_exact(&loaded.state, &receipt)?,
                errors::ACK_REPLAY_UNKNOWN
            );
            return Ok(());
        }
        let expected = loaded
            .state
            .acknowledged_sequence
            .checked_add(FIRST_SEQUENCE)
            .context(errors::ACK_ORDER)?;
        ensure!(sequence == expected, errors::ACK_ORDER);
        let head = loaded.state.pending.front().context(errors::ACK_ORDER)?;
        ensure!(
            head.sequence.0 == sequence && head.event_id == event_id,
            errors::ACK_ORDER
        );
        let receipt = match expected_witness {
            Some(expected) => {
                ensure!(
                    expected.assign_envelope(head)? == *head,
                    errors::OUTBOX_CORRUPT
                );
                let witness =
                    super::outbox_pending_witness_state::take_exact(&mut loaded.state, &expected)?;
                Some(CodexOccurrenceReceipt::from_binding(
                    witness.into_binding(),
                )?)
            }
            None => {
                ensure!(
                    !super::outbox_pending_witness_state::contains_event(&loaded.state, head)?,
                    errors::OUTBOX_CORRUPT
                );
                None
            }
        };
        let acknowledged = loaded
            .state
            .pending
            .pop_front()
            .context(errors::ACK_ORDER)?;
        loaded.state.acknowledged_sequence = sequence;
        loaded.state.acknowledged_events.push_back(acknowledged);
        outbox_validation::trim_acknowledged(&mut loaded.state);
        if let Some(receipt) = receipt {
            super::outbox_receipt_state::insert(&mut loaded.state, receipt)?;
        }
        outbox_storage::persist(&lock, &self.path, &loaded)
    }

    pub fn remove_occurrence_receipt(
        &self,
        assigned: &HookEnvelope,
        occurrence: &HookEnvelope,
        identity: &CodexOccurrenceIdentity,
    ) -> Result<()> {
        let expected = CodexOccurrenceReceipt::capture(assigned, occurrence, identity)?;
        let lock = OutboxLock::acquire(&self.directory, &self.path)?;
        let mut loaded = outbox_storage::load(&lock, &self.path)?;
        outbox_validation::check_configured(&loaded.state, self.owner.as_ref())?;
        let index = loaded.state.occurrence_receipts.iter().position(|receipt| {
            receipt.event_id() == assigned.event_id || receipt.sequence() == assigned.sequence
        });
        let Some(index) = index else {
            ensure!(
                assigned.sequence.0 <= loaded.state.acknowledged_sequence,
                errors::OUTBOX_CORRUPT
            );
            return Ok(());
        };
        ensure!(
            loaded.state.occurrence_receipts.get(index) == Some(&expected),
            errors::OUTBOX_CORRUPT
        );
        loaded.state.occurrence_receipts.remove(index);
        outbox_storage::persist(&lock, &self.path, &loaded)
    }
}
