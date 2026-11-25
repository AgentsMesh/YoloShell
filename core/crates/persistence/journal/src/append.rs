use super::*;

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) use super::append_test_support::AppendStage;

impl Journal {
    pub fn prepare_append_attempt(
        &self,
        payload_len: usize,
    ) -> Result<AppendAttempt, JournalError> {
        self.validate_append_length(payload_len)?;
        self.commit_cursor.verify_current()?;
        self.entry
            .verify_current_unchanged(self.identity, self.max_disk_bytes)?;
        Ok(self.append_attempt())
    }

    pub fn append(&mut self, payload: &[u8]) -> Result<OutputChunk, JournalError> {
        self.append_with_hook(payload, |_| Ok(()))
    }

    pub(super) fn append_with_hook<F>(
        &mut self,
        payload: &[u8],
        mut hook: F,
    ) -> Result<OutputChunk, JournalError>
    where
        F: FnMut(AppendStageValue) -> Result<(), JournalError>,
    {
        self.validate_append_length(payload.len())?;
        self.invoke_stage(AppendStageValue::BeforeWrite, &mut hook)?;
        self.commit_cursor.verify_current()?;
        self.entry
            .verify_current_unchanged(self.identity, self.max_disk_bytes)?;
        let sequence = self
            .last_sequence
            .checked_add(INITIAL_RETAINED_SEQUENCE)
            .ok_or(JournalError::SequenceExhausted)?;
        let expected_length = self
            .disk_bytes
            .checked_add(RECORD_HEADER_BYTES as u64)
            .and_then(|length| length.checked_add(payload.len() as u64))
            .ok_or(JournalError::DiskLimitExceeded)?;
        self.write_healthy = false;
        let index = write_record(
            &mut self.file,
            RecordKind::Output,
            self.runtime_epoch,
            sequence,
            payload,
        )?;
        self.file.sync_data()?;
        self.invoke_stage(AppendStageValue::AfterRecordSync, &mut hook)?;
        let identity =
            FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), self.max_disk_bytes)?;
        if !self.identity.same_object(identity) || identity.length() != expected_length {
            return Err(anyhow::anyhow!("journal identity changed while appending").into());
        }
        self.entry
            .verify_current_unchanged(identity, self.max_disk_bytes)?;
        self.commit_cursor.publish(expected_length)?;
        self.invoke_stage(AppendStageValue::AfterCursorCommit, &mut hook)?;
        self.commit_cursor.verify_current()?;
        self.entry
            .verify_current_unchanged(identity, self.max_disk_bytes)?;
        self.disk_bytes = expected_length;
        self.identity = identity;
        self.last_sequence = sequence;
        self.records.push_back(index);
        self.write_healthy = true;
        Ok(OutputChunk {
            runtime_epoch: self.runtime_epoch,
            sequence,
            data: payload.to_vec(),
        })
    }

    pub(super) fn validate_append_length(&self, payload_len: usize) -> Result<(), JournalError> {
        if !self.write_healthy {
            return Err(JournalError::WriteStateUncertain);
        }
        if payload_len > u32::MAX as usize {
            return Err(JournalError::PayloadTooLarge);
        }
        if self.would_exceed_limit(payload_len) {
            return Err(JournalError::DiskLimitExceeded);
        }
        self.identity.verify_file_bounded(
            &self.file,
            self.disk_bytes,
            self.entry.path(),
            self.max_disk_bytes,
        )?;
        Ok(())
    }

    pub fn would_exceed_limit(&self, payload_len: usize) -> bool {
        let record_bytes = (RECORD_HEADER_BYTES as u64).saturating_add(payload_len as u64);
        self.disk_bytes.saturating_add(record_bytes) > self.max_disk_bytes
    }

    pub(super) fn invoke_stage<F>(
        &mut self,
        stage: AppendStageValue,
        hook: &mut F,
    ) -> Result<(), JournalError>
    where
        F: FnMut(AppendStageValue) -> Result<(), JournalError>,
    {
        #[cfg(any(test, feature = "runtime-component-tests"))]
        if let Some(fault) = self.append_fault.as_mut()
            && fault.stage == stage
        {
            fault.remaining -= 1;
            if fault.remaining == 0 {
                self.append_fault = None;
            }
            return Err(std::io::Error::other(APPEND_FAULT_MESSAGE).into());
        }
        hook(stage)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AppendStageValue {
    BeforeWrite,
    AfterRecordSync,
    AfterCursorCommit,
}
