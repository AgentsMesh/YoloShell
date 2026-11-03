use super::*;

pub(super) struct ScanState {
    pub(super) records: VecDeque<RecordIndex>,
    pub(super) retained_from: u64,
    pub(super) last_sequence: u64,
}

pub(super) fn scan_committed(
    file: &mut File,
    committed_bytes: u64,
    runtime_epoch: Uuid,
) -> Result<ScanState, JournalError> {
    if committed_bytes < JOURNAL_HEADER_BYTES as u64 {
        return Err(JournalError::InvalidFormat);
    }
    let mut state = ScanState::new();
    let mut offset = JOURNAL_HEADER_BYTES as u64;
    while offset < committed_bytes {
        let (kind, index, checksum, record_end) =
            read_index(file, offset, committed_bytes)?.ok_or(JournalError::InvalidFormat)?;
        if checksum_payload(file, &index)? != checksum
            || (kind == RecordKind::Cursor && index.payload_len != 0)
        {
            return Err(JournalError::Corrupt {
                sequence: index.sequence,
            });
        }
        state.accept(kind, &index, runtime_epoch)?;
        offset = record_end;
    }
    if offset != committed_bytes {
        return Err(JournalError::InvalidFormat);
    }
    Ok(state)
}

impl ScanState {
    fn new() -> Self {
        Self {
            records: VecDeque::new(),
            retained_from: INITIAL_RETAINED_SEQUENCE,
            last_sequence: EMPTY_SEQUENCE,
        }
    }

    fn accept(
        &mut self,
        kind: RecordKind,
        index: &RecordIndex,
        runtime_epoch: Uuid,
    ) -> Result<(), JournalError> {
        if index.runtime_epoch != runtime_epoch {
            return Ok(());
        }
        if index.sequence < INITIAL_RETAINED_SEQUENCE {
            return Err(JournalError::Corrupt {
                sequence: index.sequence,
            });
        }
        match kind {
            RecordKind::Cursor => self.accept_cursor(index.sequence),
            RecordKind::Output if index.sequence >= self.retained_from => {
                self.accept_output(index)?
            }
            RecordKind::Output => {}
        }
        Ok(())
    }

    fn accept_cursor(&mut self, sequence: u64) {
        self.retained_from = self.retained_from.max(sequence);
        self.last_sequence = self
            .last_sequence
            .max(sequence.saturating_sub(INITIAL_RETAINED_SEQUENCE));
        self.records
            .retain(|record| record.sequence >= self.retained_from);
    }

    fn accept_output(&mut self, index: &RecordIndex) -> Result<(), JournalError> {
        let expected = self
            .last_sequence
            .checked_add(INITIAL_RETAINED_SEQUENCE)
            .ok_or(JournalError::SequenceExhausted)?;
        if index.sequence != expected {
            return Err(JournalError::Corrupt {
                sequence: index.sequence,
            });
        }
        self.last_sequence = index.sequence;
        self.records.push_back(index.clone());
        Ok(())
    }
}

fn read_index(
    file: &mut File,
    offset: u64,
    file_len: u64,
) -> Result<Option<(RecordKind, RecordIndex, u32, u64)>, JournalError> {
    let header_bytes = RECORD_HEADER_BYTES as u64;
    if file_len.saturating_sub(offset) < header_bytes {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut header = [0_u8; RECORD_HEADER_BYTES];
    file.read_exact(&mut header)?;
    let kind = match &header[..MAGIC_BYTES] {
        magic if magic == OUTPUT_MAGIC => RecordKind::Output,
        magic if magic == CURSOR_MAGIC => RecordKind::Cursor,
        _ => return Err(JournalError::InvalidFormat),
    };
    let runtime_epoch = Uuid::from_slice(&header[EPOCH_START..EPOCH_END])
        .map_err(|_| JournalError::InvalidFormat)?;
    let sequence = read_u64(&header[SEQUENCE_START..SEQUENCE_END])?;
    let payload_len = read_u32(&header[LENGTH_START..LENGTH_END])?;
    let checksum = read_u32(&header[CHECKSUM_START..CHECKSUM_END])?;
    let payload_offset = offset
        .checked_add(header_bytes)
        .ok_or(JournalError::InvalidFormat)?;
    let record_end = payload_offset
        .checked_add(u64::from(payload_len))
        .ok_or(JournalError::InvalidFormat)?;
    if record_end > file_len {
        return Ok(None);
    }
    Ok(Some((
        kind,
        RecordIndex {
            runtime_epoch,
            sequence,
            payload_offset,
            payload_len,
            checksum,
        },
        checksum,
        record_end,
    )))
}

fn read_u64(bytes: &[u8]) -> Result<u64, JournalError> {
    bytes
        .try_into()
        .map(u64::from_be_bytes)
        .map_err(|_| JournalError::InvalidFormat)
}

fn read_u32(bytes: &[u8]) -> Result<u32, JournalError> {
    bytes
        .try_into()
        .map(u32::from_be_bytes)
        .map_err(|_| JournalError::InvalidFormat)
}
