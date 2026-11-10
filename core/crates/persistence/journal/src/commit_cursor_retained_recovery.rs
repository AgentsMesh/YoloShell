use super::*;

pub(super) fn restore_retired_cursor(
    entry: &PrivateFileEntry,
    header: JournalHeader,
    file_len: u64,
) -> Result<Option<(CursorRecord, FileIdentity)>, JournalError> {
    let Some((record, identity)) = read_retired_cursor(entry)? else {
        return Ok(None);
    };
    if record.generation != header.generation
        || record.committed_bytes < header.sealed_bytes
        || record.committed_bytes > file_len
    {
        return Err(JournalError::InvalidFormat);
    }
    entry.restore_isolated_retained(identity, RETAINED_CURSOR_BYTES as u64)?;
    Ok(Some(retained::read_retained_cursor(entry)?))
}

pub(super) fn settle_retired_cursor(
    entry: &PrivateFileEntry,
    current: CursorRecord,
) -> Result<(), JournalError> {
    let Some((retired, identity)) = read_retired_cursor(entry)? else {
        return Ok(());
    };
    let same_revision = retired.publication_revision == current.publication_revision;
    if retired.generation != current.generation
        || retired.publication_revision > current.publication_revision
        || (same_revision && retired != current)
    {
        return Err(JournalError::InvalidFormat);
    }
    entry.recycle_isolated_retained(identity, RETAINED_CURSOR_BYTES as u64)?;
    Ok(())
}

fn read_retired_cursor(
    entry: &PrivateFileEntry,
) -> Result<Option<(CursorRecord, FileIdentity)>, JournalError> {
    let Some(opened) = entry.open_isolated_retained(RETAINED_CURSOR_BYTES as u64)? else {
        return Ok(None);
    };
    let identity = opened.identity();
    let record = super::slots::read_active_cursor(opened.file(), identity, entry.path())?;
    entry.verify_isolated_retained(identity, RETAINED_CURSOR_BYTES as u64)?;
    Ok(Some((record, identity)))
}
