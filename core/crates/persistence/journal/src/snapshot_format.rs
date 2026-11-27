use super::*;

pub(super) fn encode_snapshot(snapshot: &TerminalSnapshot) -> Result<Vec<u8>, JournalError> {
    if u64::try_from(snapshot.data.len()).map_err(|_| JournalError::PayloadTooLarge)?
        > MAX_SNAPSHOT_DISK_BYTES
    {
        return Err(JournalError::PayloadTooLarge);
    }
    let payload = rmp_serde::to_vec_named(snapshot).map_err(JournalError::SnapshotEncode)?;
    let total = SNAPSHOT_HEADER_BYTES
        .checked_add(payload.len())
        .ok_or(JournalError::PayloadTooLarge)?;
    if u64::try_from(total).map_err(|_| JournalError::PayloadTooLarge)? > MAX_SNAPSHOT_DISK_BYTES
        || payload.len() > u32::MAX as usize
    {
        return Err(JournalError::PayloadTooLarge);
    }
    let payload_length = u32::try_from(payload.len()).map_err(|_| JournalError::PayloadTooLarge)?;
    let mut encoded = Vec::with_capacity(total);
    encoded.extend_from_slice(SNAPSHOT_MAGIC);
    encoded.extend_from_slice(&payload_length.to_be_bytes());
    encoded.extend_from_slice(&crc32fast::hash(&payload).to_be_bytes());
    encoded.extend_from_slice(&payload);
    Ok(encoded)
}

pub(super) fn decode_snapshot(bytes: &[u8]) -> Result<TerminalSnapshot, JournalError> {
    if bytes.len() < SNAPSHOT_HEADER_BYTES || &bytes[..MAGIC_BYTES] != SNAPSHOT_MAGIC {
        return Err(JournalError::InvalidFormat);
    }
    let length = u32::from_be_bytes(
        bytes[SNAPSHOT_LENGTH_START..SNAPSHOT_LENGTH_END]
            .try_into()
            .map_err(|_| JournalError::InvalidFormat)?,
    ) as usize;
    let checksum = u32::from_be_bytes(
        bytes[SNAPSHOT_CHECKSUM_START..SNAPSHOT_CHECKSUM_END]
            .try_into()
            .map_err(|_| JournalError::InvalidFormat)?,
    );
    if SNAPSHOT_HEADER_BYTES.checked_add(length) != Some(bytes.len()) {
        return Err(JournalError::InvalidFormat);
    }
    let payload = &bytes[SNAPSHOT_HEADER_BYTES..];
    if crc32fast::hash(payload) != checksum {
        return Err(JournalError::Corrupt {
            sequence: EMPTY_SEQUENCE,
        });
    }
    rmp_serde::from_slice(payload).map_err(JournalError::SnapshotDecode)
}
