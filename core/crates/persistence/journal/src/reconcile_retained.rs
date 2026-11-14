use super::*;
use std::os::unix::fs::FileExt;

#[cfg(any(test, feature = "runtime-component-tests"))]
const RETAINED_RECONCILE_FAULT_MESSAGE: &str = "injected retained reconcile failure";

pub(super) struct ExpectedAppend {
    pub(super) base: u64,
    pub(super) end: u64,
    pub(super) sequence: u64,
    record: Vec<u8>,
}

impl Journal {
    pub fn reconcile_append_retained(
        &mut self,
        attempt: AppendAttempt,
        payload: &[u8],
    ) -> Result<Option<OutputChunk>, JournalError> {
        self.reconcile_append_retained_classified(attempt, payload)
            .into_result()
            .map(Some)
    }

    pub(super) fn reconcile_append_retained_attempt(
        &mut self,
        attempt: AppendAttempt,
        payload: &[u8],
    ) -> Result<OutputChunk, super::reconcile_retained_outcome::RetainedAppendFailure> {
        use super::reconcile_retained_outcome::RetainedAppendFailure;
        #[cfg(any(test, feature = "runtime-component-tests"))]
        if self.retained_reconcile_faults > 0 {
            self.retained_reconcile_faults -= 1;
            return Err(RetainedAppendFailure::BeforePublication(
                std::io::Error::other(RETAINED_RECONCILE_FAULT_MESSAGE).into(),
            ));
        }
        let expected = self
            .expected_retained_append(attempt, payload)
            .map_err(RetainedAppendFailure::BeforePublication)?;
        self.commit_cursor
            .verify_retained_current()
            .map_err(RetainedAppendFailure::BeforePublication)?;
        let current = self
            .verify_retained_journal(self.max_disk_bytes)
            .map_err(RetainedAppendFailure::BeforePublication)?;
        let committed = self.commit_cursor.record.committed_bytes;
        if committed == expected.end {
            self.commit_cursor
                .sync_retained_parent()
                .map_err(RetainedAppendFailure::CommitUncertain)?;
            return self
                .confirm_retained_append(attempt, payload, &expected)
                .map_err(RetainedAppendFailure::CommitUncertain);
        }
        if committed != expected.base || current.length() < expected.base {
            return Err(RetainedAppendFailure::BeforePublication(
                anyhow::anyhow!("retained commit cursor is inconsistent").into(),
            ));
        }
        let prefix = scan::scan_committed(&mut self.file, expected.base, self.runtime_epoch)
            .map_err(RetainedAppendFailure::BeforePublication)?;
        if prefix.last_sequence != attempt.previous_sequence {
            return Err(RetainedAppendFailure::BeforePublication(
                JournalError::Corrupt {
                    sequence: prefix.last_sequence,
                },
            ));
        }
        self.write_healthy = false;
        self.verify_retained_journal(self.max_disk_bytes)
            .map_err(RetainedAppendFailure::BeforePublication)?;
        if !self
            .retained_tail_matches(&expected)
            .map_err(RetainedAppendFailure::BeforePublication)?
        {
            self.rewrite_retained_tail(&expected, payload)
                .map_err(RetainedAppendFailure::BeforePublication)?;
        }
        let journal_identity = self
            .verify_exact_retained_tail(&expected)
            .map_err(RetainedAppendFailure::BeforePublication)?;
        self.commit_cursor
            .publish_retained(
                expected.end,
                &self.entry,
                journal_identity,
                self.max_disk_bytes,
            )
            .map_err(RetainedAppendFailure::CommitUncertain)?;
        self.confirm_retained_append(attempt, payload, &expected)
            .map_err(RetainedAppendFailure::CommitUncertain)
    }

    pub(super) fn expected_retained_append(
        &self,
        attempt: AppendAttempt,
        payload: &[u8],
    ) -> Result<ExpectedAppend, JournalError> {
        if self.header.generation != attempt.generation
            || self.runtime_epoch != attempt.runtime_epoch
            || self.last_sequence != attempt.previous_sequence
            || self.disk_bytes != attempt.cursor.record.committed_bytes
        {
            return Err(JournalError::InvalidFormat);
        }
        validate_retained_payload_len(payload.len())?;
        let sequence = attempt
            .previous_sequence
            .checked_add(INITIAL_RETAINED_SEQUENCE)
            .ok_or(JournalError::SequenceExhausted)?;
        let record =
            record::encode_record(RecordKind::Output, self.runtime_epoch, sequence, payload)?;
        let end = self
            .disk_bytes
            .checked_add(record.len() as u64)
            .ok_or(JournalError::DiskLimitExceeded)?;
        if end > self.max_disk_bytes {
            return Err(JournalError::DiskLimitExceeded);
        }
        Ok(ExpectedAppend {
            base: attempt.cursor.record.committed_bytes,
            end,
            sequence,
            record,
        })
    }

    fn rewrite_retained_tail(
        &mut self,
        expected: &ExpectedAppend,
        payload: &[u8],
    ) -> Result<(), JournalError> {
        self.file.set_len(expected.base)?;
        self.file.sync_data()?;
        self.verify_retained_journal(self.max_disk_bytes)?;
        self.file.seek(SeekFrom::End(0))?;
        write_record(
            &mut self.file,
            RecordKind::Output,
            self.runtime_epoch,
            expected.sequence,
            payload,
        )?;
        self.file.sync_data()?;
        self.verify_retained_journal(self.max_disk_bytes)?;
        Ok(())
    }

    pub(super) fn verify_exact_retained_tail(
        &mut self,
        expected: &ExpectedAppend,
    ) -> Result<FileIdentity, JournalError> {
        let identity = self.verify_retained_journal(self.max_disk_bytes)?;
        if identity.length() != expected.end || !self.retained_tail_matches(expected)? {
            return Err(anyhow::anyhow!("retained journal tail changed").into());
        }
        Ok(identity)
    }

    fn retained_tail_matches(&self, expected: &ExpectedAppend) -> Result<bool, JournalError> {
        let length = self.file.metadata()?.len();
        if length != expected.end {
            return Ok(false);
        }
        let mut actual = vec![0_u8; expected.record.len()];
        self.file.read_exact_at(&mut actual, expected.base)?;
        Ok(actual == expected.record)
    }

    pub(super) fn verify_retained_journal(
        &self,
        max_bytes: u64,
    ) -> Result<FileIdentity, JournalError> {
        let identity =
            FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), max_bytes)?;
        if !self.identity.same_object(identity) {
            return Err(anyhow::anyhow!("retained journal descriptor changed").into());
        }
        self.entry.verify_retained_unchanged(identity, max_bytes)?;
        Ok(identity)
    }
}

pub(in crate::journal) fn validate_retained_payload_len(
    payload_len: usize,
) -> Result<(), JournalError> {
    if payload_len > u32::MAX as usize {
        return Err(JournalError::PayloadTooLarge);
    }
    Ok(())
}
