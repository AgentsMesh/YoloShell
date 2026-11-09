impl Journal {
    fn confirm_retained_batch(
        &mut self,
        attempt: AppendAttempt,
        payloads: &[&[u8]],
        expected: &ExpectedBatch,
    ) -> Result<Vec<OutputChunk>, JournalError> {
        self.commit_cursor.verify_retained_current()?;
        #[cfg(any(test, feature = "runtime-component-tests"))]
        super::tests_retained_hooks::run_confirm_hook()?;
        if self.commit_cursor.record.committed_bytes != expected.end {
            return Err(anyhow::anyhow!("retained cursor did not commit expected batch").into());
        }
        let identity = self.verify_exact_retained_batch(expected)?;
        let state = scan::scan_committed(&mut self.file, expected.end, self.runtime_epoch)?;
        let tail: Vec<_> = state
            .records
            .iter()
            .filter(|record| record.sequence > attempt.previous_sequence)
            .collect();
        if state.last_sequence != *expected.sequences.last().unwrap_or(&EMPTY_SEQUENCE)
            || tail.len() != payloads.len()
        {
            return Err(JournalError::Corrupt {
                sequence: state.last_sequence,
            });
        }
        self.entry
            .verify_retained_unchanged(identity, self.max_disk_bytes)?;
        let mut chunks = Vec::with_capacity(payloads.len());
        for ((record, payload), sequence) in tail
            .into_iter()
            .zip(payloads.iter())
            .zip(expected.sequences.iter().copied())
        {
            let data = read_payload(&mut self.file, record)?;
            if record.sequence != sequence || data.as_slice() != *payload {
                return Err(JournalError::Corrupt {
                    sequence: record.sequence,
                });
            }
            chunks.push(OutputChunk {
                runtime_epoch: attempt.runtime_epoch,
                sequence,
                data,
            });
        }
        self.identity = identity;
        self.records = state.records;
        self.retained_from_sequence = state.retained_from;
        self.last_sequence = state.last_sequence;
        self.disk_bytes = expected.end;
        self.write_healthy = true;
        Ok(chunks)
    }

    pub(super) fn prove_retained_batch_uncommitted(
        &mut self,
        attempt: AppendAttempt,
        payloads: &[&[u8]],
    ) -> Result<(), JournalError> {
        let expected = self.expected_retained_batch(attempt, payloads)?;
        let _lock = self.entry.lock_retained_parent()?;
        self.commit_cursor
            .verify_retained_evidence(attempt.cursor)?;
        let before = self.verify_retained_journal(self.max_disk_bytes)?;
        if self.commit_cursor.record.committed_bytes != expected.base
            || before.length() < expected.base
        {
            return Err(anyhow::anyhow!(RETAINED_BATCH_UNCOMMITTED_PROOF_DETAIL).into());
        }
        let prefix = scan::scan_committed(&mut self.file, expected.base, self.runtime_epoch)?;
        if prefix.last_sequence != attempt.previous_sequence {
            return Err(JournalError::Corrupt {
                sequence: prefix.last_sequence,
            });
        }
        self.entry.sync_retained_parent()?;
        self.commit_cursor
            .verify_retained_evidence(attempt.cursor)?;
        let after = self.verify_retained_journal(self.max_disk_bytes)?;
        if !before.unchanged(after) {
            return Err(anyhow::anyhow!(RETAINED_BATCH_UNCOMMITTED_PROOF_DETAIL).into());
        }
        Ok(())
    }
}
