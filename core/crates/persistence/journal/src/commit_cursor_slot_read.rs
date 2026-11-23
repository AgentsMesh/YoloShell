use super::*;
use std::os::unix::fs::FileExt;

pub(crate) fn read_slot(
    file: &File,
    length: u64,
    index: usize,
    tolerate_corrupt: bool,
) -> Result<Option<CursorRecord>, JournalError> {
    let offset = slot_offset(index)?;
    let end = offset
        .checked_add(COMMIT_CURSOR_BYTES as u64)
        .ok_or(JournalError::InvalidFormat)?;
    if length < end {
        return Ok(None);
    }
    let mut bytes = [0_u8; COMMIT_CURSOR_BYTES];
    file.read_exact_at(&mut bytes, offset)?;
    match decode_cursor(&bytes) {
        Ok(record) => Ok(Some(record)),
        Err(_) if tolerate_corrupt => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn slot_offset(index: usize) -> Result<u64, JournalError> {
    u64::try_from(
        index
            .checked_mul(COMMIT_CURSOR_BYTES)
            .ok_or(JournalError::InvalidFormat)?,
    )
    .map_err(|_| JournalError::InvalidFormat)
}
