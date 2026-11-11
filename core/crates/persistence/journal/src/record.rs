use super::*;

pub(super) fn write_record(
    file: &mut File,
    kind: RecordKind,
    runtime_epoch: Uuid,
    sequence: u64,
    payload: &[u8],
) -> Result<RecordIndex, JournalError> {
    let encoded = encode_record(kind, runtime_epoch, sequence, payload)?;
    let checksum = checksum(payload);
    let offset = file.seek(SeekFrom::End(0))?;
    file.write_all(&encoded)?;
    file.flush()?;
    Ok(RecordIndex {
        runtime_epoch,
        sequence,
        payload_offset: offset + RECORD_HEADER_BYTES as u64,
        payload_len: payload.len() as u32,
        checksum,
    })
}

pub(super) fn encode_record(
    kind: RecordKind,
    runtime_epoch: Uuid,
    sequence: u64,
    payload: &[u8],
) -> Result<Vec<u8>, JournalError> {
    if payload.len() > u32::MAX as usize {
        return Err(JournalError::PayloadTooLarge);
    }
    let magic = match kind {
        RecordKind::Output => OUTPUT_MAGIC,
        RecordKind::Cursor => CURSOR_MAGIC,
    };
    let mut encoded = Vec::with_capacity(RECORD_HEADER_BYTES + payload.len());
    encoded.extend_from_slice(magic);
    encoded.extend_from_slice(runtime_epoch.as_bytes());
    encoded.extend_from_slice(&sequence.to_be_bytes());
    encoded.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    encoded.extend_from_slice(&checksum(payload).to_be_bytes());
    encoded.extend_from_slice(payload);
    Ok(encoded)
}

fn checksum(payload: &[u8]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(payload);
    hasher.finalize()
}

pub(super) fn read_payload(file: &mut File, record: &RecordIndex) -> Result<Vec<u8>, JournalError> {
    file.seek(SeekFrom::Start(record.payload_offset))?;
    let mut payload = vec![0_u8; record.payload_len as usize];
    file.read_exact(&mut payload)?;
    Ok(payload)
}

pub(super) fn checksum_payload(file: &mut File, record: &RecordIndex) -> Result<u32, JournalError> {
    file.seek(SeekFrom::Start(record.payload_offset))?;
    let mut remaining = u64::from(record.payload_len);
    let mut buffer = [0_u8; CHECKSUM_BUFFER_BYTES];
    let mut hasher = Hasher::new();
    while remaining > 0 {
        let count = usize::try_from(remaining.min(CHECKSUM_BUFFER_BYTES as u64))
            .map_err(|_| JournalError::PayloadTooLarge)?;
        file.read_exact(&mut buffer[..count])?;
        hasher.update(&buffer[..count]);
        remaining -= count as u64;
    }
    Ok(hasher.finalize())
}
