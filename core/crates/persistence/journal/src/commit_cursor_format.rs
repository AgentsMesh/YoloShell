use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CursorRecord {
    pub(super) generation: Uuid,
    pub(super) committed_bytes: u64,
    pub(super) publication_revision: u64,
}

pub(super) fn encode_cursor(record: CursorRecord) -> [u8; COMMIT_CURSOR_BYTES] {
    let mut bytes = [0_u8; COMMIT_CURSOR_BYTES];
    bytes[..MAGIC_BYTES].copy_from_slice(COMMIT_CURSOR_MAGIC);
    bytes[GENERATION_START..GENERATION_END].copy_from_slice(record.generation.as_bytes());
    bytes[DURABLE_LENGTH_START..DURABLE_LENGTH_END]
        .copy_from_slice(&record.committed_bytes.to_be_bytes());
    bytes[CURSOR_REVISION_START..CURSOR_REVISION_END]
        .copy_from_slice(&record.publication_revision.to_be_bytes());
    let checksum = crc32fast::hash(&bytes[..CURSOR_CHECKSUM_START]);
    bytes[CURSOR_CHECKSUM_START..CURSOR_CHECKSUM_END].copy_from_slice(&checksum.to_be_bytes());
    bytes
}

pub(super) fn decode_cursor(bytes: &[u8]) -> Result<CursorRecord, JournalError> {
    if bytes.len() != COMMIT_CURSOR_BYTES || &bytes[..MAGIC_BYTES] != COMMIT_CURSOR_MAGIC {
        return Err(JournalError::InvalidFormat);
    }
    let stored = u32::from_be_bytes(
        bytes[CURSOR_CHECKSUM_START..CURSOR_CHECKSUM_END]
            .try_into()
            .map_err(|_| JournalError::InvalidFormat)?,
    );
    if crc32fast::hash(&bytes[..CURSOR_CHECKSUM_START]) != stored {
        return Err(JournalError::InvalidFormat);
    }
    Ok(CursorRecord {
        generation: Uuid::from_slice(&bytes[GENERATION_START..GENERATION_END])
            .map_err(|_| JournalError::InvalidFormat)?,
        committed_bytes: u64::from_be_bytes(
            bytes[DURABLE_LENGTH_START..DURABLE_LENGTH_END]
                .try_into()
                .map_err(|_| JournalError::InvalidFormat)?,
        ),
        publication_revision: u64::from_be_bytes(
            bytes[CURSOR_REVISION_START..CURSOR_REVISION_END]
                .try_into()
                .map_err(|_| JournalError::InvalidFormat)?,
        ),
    })
}
