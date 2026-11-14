use super::append::AppendStageValue;
use super::*;

impl Journal {
    pub fn batch_would_exceed_limit(&self, payload_bytes: usize, payload_count: usize) -> bool {
        let record_bytes = (RECORD_HEADER_BYTES as u64)
            .saturating_mul(payload_count as u64)
            .saturating_add(payload_bytes as u64);
        self.disk_bytes.saturating_add(record_bytes) > self.max_disk_bytes
    }

    /// Append several output records under one durable barrier.
    ///
    /// Every record is written before the file is synced and the commit cursor
    /// is published once for the complete batch. If publication fails, the
    /// journal remains unhealthy and the normal retained reconciliation path
    /// is required before another write.
    pub fn append_batch(&mut self, payloads: &[&[u8]]) -> Result<Vec<OutputChunk>, JournalError> {
        if payloads.is_empty() {
            return Ok(Vec::new());
        }
        if !self.write_healthy {
            return Err(JournalError::WriteStateUncertain);
        }
        self.commit_cursor.verify_current()?;
        self.entry
            .verify_current_unchanged(self.identity, self.max_disk_bytes)?;
        self.invoke_stage(AppendStageValue::BeforeWrite, &mut |_| Ok(()))?;
        let (expected_length, sequences) = self.validate_batch(payloads)?;
        self.write_healthy = false;
        let mut indexes = Vec::with_capacity(payloads.len());
        for (payload, sequence) in payloads.iter().zip(sequences.iter().copied()) {
            indexes.push(write_record(
                &mut self.file,
                RecordKind::Output,
                self.runtime_epoch,
                sequence,
                payload,
            )?);
        }
        self.file.sync_data()?;
        self.invoke_stage(AppendStageValue::AfterRecordSync, &mut |_| Ok(()))?;
        let identity =
            FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), self.max_disk_bytes)?;
        if !self.identity.same_object(identity) || identity.length() != expected_length {
            return Err(anyhow::anyhow!("journal identity changed while appending batch").into());
        }
        self.entry
            .verify_current_unchanged(identity, self.max_disk_bytes)?;
        self.commit_cursor.publish(expected_length)?;
        self.invoke_stage(AppendStageValue::AfterCursorCommit, &mut |_| Ok(()))?;
        self.commit_cursor.verify_current()?;
        self.disk_bytes = expected_length;
        self.identity = identity;
        self.last_sequence = *sequences.last().expect("non-empty batch");
        self.records.extend(indexes);
        self.write_healthy = true;
        Ok(payloads
            .iter()
            .zip(sequences)
            .map(|(payload, sequence)| OutputChunk {
                runtime_epoch: self.runtime_epoch,
                sequence,
                data: payload.to_vec(),
            })
            .collect())
    }

    fn validate_batch(&self, payloads: &[&[u8]]) -> Result<(u64, Vec<u64>), JournalError> {
        let mut expected_length = self.disk_bytes;
        let mut previous = self.last_sequence;
        let mut sequences = Vec::with_capacity(payloads.len());
        for payload in payloads {
            if payload.len() > u32::MAX as usize {
                return Err(JournalError::PayloadTooLarge);
            }
            let record_bytes = (RECORD_HEADER_BYTES as u64)
                .checked_add(payload.len() as u64)
                .ok_or(JournalError::DiskLimitExceeded)?;
            expected_length = expected_length
                .checked_add(record_bytes)
                .ok_or(JournalError::DiskLimitExceeded)?;
            if expected_length > self.max_disk_bytes {
                return Err(JournalError::DiskLimitExceeded);
            }
            previous = previous
                .checked_add(INITIAL_RETAINED_SEQUENCE)
                .ok_or(JournalError::SequenceExhausted)?;
            sequences.push(previous);
        }
        self.identity.verify_file_bounded(
            &self.file,
            self.disk_bytes,
            self.entry.path(),
            self.max_disk_bytes,
        )?;
        Ok((expected_length, sequences))
    }
}
