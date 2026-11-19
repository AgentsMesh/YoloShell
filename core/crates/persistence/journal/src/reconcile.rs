use super::*;

#[derive(Clone, Copy, Debug)]
pub struct AppendAttempt {
    pub(super) generation: Uuid,
    pub(super) runtime_epoch: Uuid,
    pub(super) previous_sequence: u64,
    pub(super) cursor: commit_cursor::CommitCursorEvidence,
}

impl Journal {
    pub fn reconcile_append_batch(
        &mut self,
        attempt: AppendAttempt,
        payloads: &[&[u8]],
    ) -> Result<Option<Vec<OutputChunk>>, JournalError> {
        self.reconcile()?;
        if self.header.generation != attempt.generation
            || self.runtime_epoch != attempt.runtime_epoch
        {
            return Err(JournalError::InvalidFormat);
        }
        if self.last_sequence == attempt.previous_sequence {
            return Ok(None);
        }
        let batch_sequences = (payloads.len() as u64)
            .checked_mul(INITIAL_RETAINED_SEQUENCE)
            .ok_or(JournalError::SequenceExhausted)?;
        let expected = attempt
            .previous_sequence
            .checked_add(batch_sequences)
            .ok_or(JournalError::SequenceExhausted)?;
        if self.last_sequence != expected {
            return Err(JournalError::Corrupt {
                sequence: self.last_sequence,
            });
        }
        let byte_limit = payloads
            .iter()
            .try_fold(0_usize, |total, payload| total.checked_add(payload.len()))
            .ok_or(JournalError::PayloadTooLarge)?;
        let recovered = self.replay_after(attempt.previous_sequence, byte_limit)?;
        if recovered.len() != payloads.len()
            || recovered
                .iter()
                .zip(payloads.iter())
                .any(|(chunk, payload)| chunk.data != *payload)
        {
            return Err(JournalError::Corrupt { sequence: expected });
        }
        Ok(Some(recovered))
    }

    pub fn append_attempt(&self) -> AppendAttempt {
        AppendAttempt {
            generation: self.header.generation,
            runtime_epoch: self.runtime_epoch,
            previous_sequence: self.last_sequence,
            cursor: self.commit_cursor.evidence(),
        }
    }

    pub fn requires_reconcile(&self) -> bool {
        !self.write_healthy
    }

    pub fn reconcile(&mut self) -> Result<(), JournalError> {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        if self.reconcile_faults > 0 {
            self.reconcile_faults -= 1;
            return Err(std::io::Error::other(RECONCILE_FAULT_MESSAGE).into());
        }
        let replacement = Self::open_entry_with_max_bytes(
            self.entry.reopen()?,
            self.runtime_epoch,
            self.max_disk_bytes,
        )?;
        *self = replacement;
        Ok(())
    }

    pub fn reconcile_append(
        &mut self,
        attempt: AppendAttempt,
        payload: &[u8],
    ) -> Result<Option<OutputChunk>, JournalError> {
        self.reconcile()?;
        self.reconciled_append(attempt, payload)
    }

    pub(super) fn reconciled_append(
        &mut self,
        attempt: AppendAttempt,
        payload: &[u8],
    ) -> Result<Option<OutputChunk>, JournalError> {
        if self.header.generation != attempt.generation
            || self.runtime_epoch != attempt.runtime_epoch
        {
            return Err(JournalError::InvalidFormat);
        }
        if self.last_sequence == attempt.previous_sequence {
            return Ok(None);
        }
        let expected = attempt
            .previous_sequence
            .checked_add(INITIAL_RETAINED_SEQUENCE)
            .ok_or(JournalError::SequenceExhausted)?;
        if self.last_sequence != expected {
            return Err(JournalError::Corrupt {
                sequence: self.last_sequence,
            });
        }
        let mut recovered = self.replay_after(attempt.previous_sequence, payload.len())?;
        if recovered.len() != 1 {
            return Err(JournalError::Corrupt { sequence: expected });
        }
        let chunk = recovered.remove(0);
        if chunk.runtime_epoch != attempt.runtime_epoch
            || chunk.sequence != expected
            || chunk.data != payload
        {
            return Err(JournalError::Corrupt { sequence: expected });
        }
        Ok(Some(chunk))
    }
}
