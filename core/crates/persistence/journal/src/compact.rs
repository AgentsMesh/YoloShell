use super::*;

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CompactStage {
    TemporarySynced,
    Installed,
    BeforeReopen,
}

impl Journal {
    pub fn compact_through(&mut self, sequence: u64) -> Result<(), JournalError> {
        self.compact_with_hook(sequence, |_| Ok(()))
    }

    fn compact_with_hook<F>(&mut self, sequence: u64, mut hook: F) -> Result<(), JournalError>
    where
        F: FnMut(CompactStageValue) -> Result<(), JournalError>,
    {
        if !self.write_healthy {
            return Err(JournalError::WriteStateUncertain);
        }
        let directory_lock = self
            .entry
            .lock_and_cleanup(COMPACT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
        self.verify_source()?;
        let through = sequence.min(self.last_sequence);
        let retained_from = self
            .retained_from_sequence
            .max(through.saturating_add(INITIAL_RETAINED_SEQUENCE));
        let retained: Vec<RecordIndex> = self
            .records
            .iter()
            .filter(|record| record.sequence >= retained_from)
            .cloned()
            .collect();
        let sealed_bytes = self.compacted_size(&retained)?;
        let mut temporary = self
            .entry
            .temporary(COMPACT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
        let result = self.build_and_install(
            &mut temporary,
            retained_from,
            sealed_bytes,
            &retained,
            &mut hook,
        );
        if let Err(error) = result {
            self.entry.remove_temporary(&temporary)?;
            return Err(error);
        }
        drop(directory_lock);
        hook(CompactStageValue::BeforeReopen)?;
        let mut replacement = Self::open_entry_with_max_bytes(
            self.entry.reopen()?,
            self.runtime_epoch,
            self.max_disk_bytes,
        )?;
        if !replacement
            .identity
            .same_content_version(temporary.identity())
        {
            return Err(anyhow::anyhow!("installed journal identity changed").into());
        }
        replacement.write_healthy = true;
        *self = replacement;
        Ok(())
    }

    fn build_and_install<F>(
        &mut self,
        temporary: &mut crate::paths::private_file::TemporaryPrivateFile,
        retained_from: u64,
        sealed_bytes: u64,
        retained: &[RecordIndex],
        hook: &mut F,
    ) -> Result<(), JournalError>
    where
        F: FnMut(CompactStageValue) -> Result<(), JournalError>,
    {
        header::write_header(temporary.file_mut(), JournalHeader::new(sealed_bytes))?;
        write_record(
            temporary.file_mut(),
            RecordKind::Cursor,
            self.runtime_epoch,
            retained_from,
            &[],
        )?;
        for record in retained {
            let payload = read_payload(&mut self.file, record)?;
            if crc32fast::hash(&payload) != record.checksum {
                return Err(JournalError::Corrupt {
                    sequence: record.sequence,
                });
            }
            write_record(
                temporary.file_mut(),
                RecordKind::Output,
                self.runtime_epoch,
                record.sequence,
                &payload,
            )?;
        }
        temporary.sync_and_inspect(self.entry.path(), self.max_disk_bytes)?;
        if temporary.identity().length() != sealed_bytes {
            return Err(JournalError::InvalidFormat);
        }
        hook(CompactStageValue::TemporarySynced)?;
        self.verify_source()?;
        self.entry
            .verify_current_unchanged(self.identity, self.max_disk_bytes)?;
        self.write_healthy = false;
        self.entry
            .rename_temporary(temporary, self.max_disk_bytes)?;
        hook(CompactStageValue::Installed)?;
        self.entry
            .verify_current_content_version(temporary.identity(), self.max_disk_bytes)?;
        Ok(())
    }

    fn verify_source(&self) -> Result<(), JournalError> {
        self.identity.verify_file_bounded(
            &self.file,
            self.disk_bytes,
            self.entry.path(),
            self.max_disk_bytes,
        )?;
        self.commit_cursor.verify_current()?;
        Ok(())
    }

    fn compacted_size(&self, retained: &[RecordIndex]) -> Result<u64, JournalError> {
        let mut bytes = (JOURNAL_HEADER_BYTES as u64)
            .checked_add(RECORD_HEADER_BYTES as u64)
            .ok_or(JournalError::DiskLimitExceeded)?;
        for record in retained {
            bytes = bytes
                .checked_add(RECORD_HEADER_BYTES as u64)
                .and_then(|total| total.checked_add(u64::from(record.payload_len)))
                .ok_or(JournalError::DiskLimitExceeded)?;
        }
        if bytes > self.max_disk_bytes {
            return Err(JournalError::DiskLimitExceeded);
        }
        Ok(bytes)
    }

    #[cfg(test)]
    pub(super) fn compact_with_test_hook<F>(
        &mut self,
        sequence: u64,
        mut hook: F,
    ) -> Result<(), JournalError>
    where
        F: FnMut(CompactStage) -> Result<(), JournalError>,
    {
        self.compact_with_hook(sequence, |stage| hook(stage.into()))
    }
}

#[derive(Clone, Copy)]
enum CompactStageValue {
    TemporarySynced,
    Installed,
    BeforeReopen,
}

#[cfg(test)]
impl From<CompactStageValue> for CompactStage {
    fn from(value: CompactStageValue) -> Self {
        match value {
            CompactStageValue::TemporarySynced => Self::TemporarySynced,
            CompactStageValue::Installed => Self::Installed,
            CompactStageValue::BeforeReopen => Self::BeforeReopen,
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests_compact_wave5.rs"]
mod wave5_tests;
