use super::*;
use std::os::unix::fs::FileExt;

const RETAINED_BATCH_UNCOMMITTED_PROOF_DETAIL: &str =
    "retained batch append cannot be proven uncommitted";

pub(super) struct ExpectedBatch {
    pub(super) base: u64,
    pub(super) end: u64,
    pub(super) sequences: Vec<u64>,
    pub(super) records: Vec<Vec<u8>>,
}

impl Journal {
    pub fn reconcile_append_retained_batch(
        &mut self,
        attempt: AppendAttempt,
        payloads: &[&[u8]],
    ) -> Result<Vec<OutputChunk>, JournalError> {
        self.reconcile_append_retained_batch_classified(attempt, payloads)
            .into_result()
    }

    pub(super) fn reconcile_append_retained_batch_attempt(
        &mut self,
        attempt: AppendAttempt,
        payloads: &[&[u8]],
    ) -> Result<Vec<OutputChunk>, super::reconcile_retained_outcome::RetainedAppendBatchFailure>
    {
        use super::reconcile_retained_outcome::RetainedAppendBatchFailure;
        #[cfg(any(test, feature = "runtime-component-tests"))]
        if self.retained_reconcile_faults > 0 {
            self.retained_reconcile_faults -= 1;
            return Err(RetainedAppendBatchFailure::BeforePublication(
                std::io::Error::other("injected retained reconcile failure").into(),
            ));
        }
        let expected = self
            .expected_retained_batch(attempt, payloads)
            .map_err(RetainedAppendBatchFailure::BeforePublication)?;
        self.commit_cursor
            .verify_retained_current()
            .map_err(RetainedAppendBatchFailure::BeforePublication)?;
        let current = self
            .verify_retained_journal(self.max_disk_bytes)
            .map_err(RetainedAppendBatchFailure::BeforePublication)?;
        let committed = self.commit_cursor.record.committed_bytes;
        if committed == expected.end {
            self.commit_cursor
                .sync_retained_parent()
                .map_err(RetainedAppendBatchFailure::CommitUncertain)?;
            return self
                .confirm_retained_batch(attempt, payloads, &expected)
                .map_err(RetainedAppendBatchFailure::CommitUncertain);
        }
        if committed != expected.base || current.length() < expected.base {
            return Err(RetainedAppendBatchFailure::BeforePublication(
                anyhow::anyhow!("retained commit cursor is inconsistent").into(),
            ));
        }
        self.write_healthy = false;
        self.file
            .set_len(expected.base)
            .map_err(|error| RetainedAppendBatchFailure::BeforePublication(error.into()))?;
        self.file
            .sync_data()
            .map_err(|error| RetainedAppendBatchFailure::BeforePublication(error.into()))?;
        self.verify_retained_journal(self.max_disk_bytes)
            .map_err(RetainedAppendBatchFailure::BeforePublication)?;
        self.file
            .seek(SeekFrom::End(0))
            .map_err(|error| RetainedAppendBatchFailure::BeforePublication(error.into()))?;
        for (sequence, payload) in expected
            .sequences
            .iter()
            .copied()
            .zip(payloads.iter().copied())
        {
            write_record(
                &mut self.file,
                RecordKind::Output,
                self.runtime_epoch,
                sequence,
                payload,
            )
            .map_err(RetainedAppendBatchFailure::BeforePublication)?;
        }
        self.file
            .sync_data()
            .map_err(|error| RetainedAppendBatchFailure::BeforePublication(error.into()))?;
        let identity = self
            .verify_exact_retained_batch(&expected)
            .map_err(RetainedAppendBatchFailure::BeforePublication)?;
        self.commit_cursor
            .publish_retained(expected.end, &self.entry, identity, self.max_disk_bytes)
            .map_err(RetainedAppendBatchFailure::CommitUncertain)?;
        self.confirm_retained_batch(attempt, payloads, &expected)
            .map_err(RetainedAppendBatchFailure::CommitUncertain)
    }

    fn expected_retained_batch(
        &self,
        attempt: AppendAttempt,
        payloads: &[&[u8]],
    ) -> Result<ExpectedBatch, JournalError> {
        if payloads.is_empty()
            || self.header.generation != attempt.generation
            || self.runtime_epoch != attempt.runtime_epoch
            || self.last_sequence != attempt.previous_sequence
            || self.disk_bytes != attempt.cursor.record.committed_bytes
        {
            return Err(JournalError::InvalidFormat);
        }
        let mut end = self.disk_bytes;
        let mut previous = self.last_sequence;
        let mut sequences = Vec::with_capacity(payloads.len());
        let mut records = Vec::with_capacity(payloads.len());
        for payload in payloads {
            super::reconcile_retained::validate_retained_payload_len(payload.len())?;
            previous = previous
                .checked_add(INITIAL_RETAINED_SEQUENCE)
                .ok_or(JournalError::SequenceExhausted)?;
            let record =
                record::encode_record(RecordKind::Output, self.runtime_epoch, previous, payload)?;
            end = end
                .checked_add(record.len() as u64)
                .ok_or(JournalError::DiskLimitExceeded)?;
            if end > self.max_disk_bytes {
                return Err(JournalError::DiskLimitExceeded);
            }
            sequences.push(previous);
            records.push(record);
        }
        Ok(ExpectedBatch {
            base: self.disk_bytes,
            end,
            sequences,
            records,
        })
    }

    fn verify_exact_retained_batch(
        &mut self,
        expected: &ExpectedBatch,
    ) -> Result<FileIdentity, JournalError> {
        let identity = self.verify_retained_journal(self.max_disk_bytes)?;
        if identity.length() != expected.end || !self.retained_batch_matches(expected)? {
            return Err(anyhow::anyhow!("retained journal batch tail changed").into());
        }
        Ok(identity)
    }

    fn retained_batch_matches(&self, expected: &ExpectedBatch) -> Result<bool, JournalError> {
        let length = self.file.metadata()?.len();
        if length != expected.end {
            return Ok(false);
        }
        let mut actual = vec![0_u8; expected.end.saturating_sub(expected.base) as usize];
        self.file.read_exact_at(&mut actual, expected.base)?;
        Ok(actual
            == expected
                .records
                .iter()
                .flat_map(|record| record.iter().copied())
                .collect::<Vec<_>>())
    }
}

include!("reconcile_retained_batch_confirmation.rs");
