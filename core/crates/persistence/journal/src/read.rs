use super::*;

impl Journal {
    pub fn runtime_epoch(&self) -> Uuid {
        self.runtime_epoch
    }

    pub fn retained_from_sequence(&self) -> u64 {
        self.retained_from_sequence
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn disk_bytes(&self) -> Result<u64, JournalError> {
        self.identity.verify_file_bounded(
            &self.file,
            self.disk_bytes,
            self.entry.path(),
            self.max_disk_bytes,
        )?;
        Ok(self.disk_bytes)
    }

    pub fn max_disk_bytes(&self) -> u64 {
        self.max_disk_bytes
    }

    pub fn replay_after(
        &mut self,
        sequence: u64,
        byte_limit: usize,
    ) -> Result<Vec<OutputChunk>, JournalError> {
        self.identity.verify_file_bounded(
            &self.file,
            self.disk_bytes,
            self.entry.path(),
            self.max_disk_bytes,
        )?;
        let selected: Vec<RecordIndex> = self
            .records
            .iter()
            .filter(|record| record.sequence > sequence)
            .cloned()
            .collect();
        let mut used = 0_usize;
        let mut chunks = Vec::new();
        for record in selected {
            let payload_len = record.payload_len as usize;
            let Some(next_used) = used.checked_add(payload_len) else {
                break;
            };
            if next_used > byte_limit {
                break;
            }
            let data = read_payload(&mut self.file, &record)?;
            if crc32fast::hash(&data) != record.checksum {
                return Err(JournalError::Corrupt {
                    sequence: record.sequence,
                });
            }
            used = next_used;
            chunks.push(OutputChunk {
                runtime_epoch: self.runtime_epoch,
                sequence: record.sequence,
                data,
            });
        }
        self.identity.verify_file_bounded(
            &self.file,
            self.disk_bytes,
            self.entry.path(),
            self.max_disk_bytes,
        )?;
        self.entry
            .verify_current_unchanged(self.identity, self.max_disk_bytes)?;
        Ok(chunks)
    }
}
