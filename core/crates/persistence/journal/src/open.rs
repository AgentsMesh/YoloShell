use super::commit_cursor::cursor_exists;
use super::header::{read_header, write_header};
use super::scan::scan_committed;
use super::*;

impl Journal {
    pub fn open(path: impl AsRef<Path>, runtime_epoch: Uuid) -> Result<Self, JournalError> {
        Self::open_with_max_bytes(path, runtime_epoch, DEFAULT_MAX_DISK_BYTES)
    }

    pub fn open_with_max_bytes(
        path: impl AsRef<Path>,
        runtime_epoch: Uuid,
        max_disk_bytes: u64,
    ) -> Result<Self, JournalError> {
        let path = path.as_ref();
        let entry = PrivateFileEntry::ensure(path)?;
        Self::open_entry_with_max_bytes(entry, runtime_epoch, max_disk_bytes)
    }

    pub fn open_retained(
        entry: PrivateFileEntry,
        runtime_epoch: Uuid,
    ) -> Result<Self, JournalError> {
        Self::open_entry_with_max_bytes(entry, runtime_epoch, DEFAULT_MAX_DISK_BYTES)
    }

    pub(super) fn open_entry_with_max_bytes(
        entry: PrivateFileEntry,
        runtime_epoch: Uuid,
        max_disk_bytes: u64,
    ) -> Result<Self, JournalError> {
        validate_limit(max_disk_bytes)?;
        let cursor_entry = entry.sibling_with_suffix(COMMIT_FILE_SUFFIX)?;
        let opened = {
            let _lock = entry.lock_and_cleanup(COMPACT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
            entry.open_rw_or_create(max_disk_bytes)?
        };
        let (mut file, identity) = opened.into_parts();
        let (header, mut identity) = prepare_header(
            &entry,
            &mut file,
            identity,
            cursor_exists(&cursor_entry)?,
            max_disk_bytes,
        )?;
        let loaded_cursor = LoadedCursor::load(cursor_entry, header, identity.length())?;
        let committed_bytes = loaded_cursor.committed_bytes();
        let state = scan_committed(&mut file, committed_bytes, runtime_epoch)?;
        if identity.length() > committed_bytes {
            truncate_uncommitted(&mut file, committed_bytes)?;
            identity = inspect_journal(&entry, &file, identity, max_disk_bytes)?;
        }
        if loaded_cursor.requires_repair() {
            entry.sync_parent()?;
        }
        let commit_cursor = loaded_cursor.finish()?;
        identity = inspect_journal(&entry, &file, identity, max_disk_bytes)?;
        file.seek(SeekFrom::End(0))?;
        Ok(Self {
            entry,
            file,
            identity,
            header,
            commit_cursor,
            runtime_epoch,
            records: state.records,
            retained_from_sequence: state.retained_from,
            last_sequence: state.last_sequence,
            disk_bytes: identity.length(),
            max_disk_bytes,
            write_healthy: true,
            #[cfg(any(test, feature = "runtime-component-tests"))]
            append_fault: None,
            #[cfg(any(test, feature = "runtime-component-tests"))]
            reconcile_faults: 0,
            #[cfg(any(test, feature = "runtime-component-tests"))]
            retained_reconcile_faults: 0,
        })
    }
}

fn validate_limit(max_disk_bytes: u64) -> Result<(), JournalError> {
    if max_disk_bytes > MAX_JOURNAL_DISK_BYTES || max_disk_bytes < JOURNAL_HEADER_BYTES as u64 {
        return Err(JournalError::DiskLimitExceeded);
    }
    Ok(())
}

fn prepare_header(
    entry: &PrivateFileEntry,
    file: &mut File,
    identity: FileIdentity,
    cursor_present: bool,
    max_disk_bytes: u64,
) -> Result<(JournalHeader, FileIdentity), JournalError> {
    let length = identity.length();
    let recoverable_initialization = !cursor_present && length <= JOURNAL_HEADER_BYTES as u64;
    if length > 0 {
        match read_header(file) {
            Ok(header) => return Ok((header, identity)),
            Err(_) if !recoverable_initialization => return Err(JournalError::InvalidFormat),
            Err(_) => {}
        }
    }
    file.set_len(0)?;
    let header = JournalHeader::new(JOURNAL_HEADER_BYTES as u64);
    write_header(file, header)?;
    file.sync_all()?;
    let updated = FileIdentity::inspect_file_bounded(file, entry.path(), max_disk_bytes)?;
    if !identity.same_object(updated) || updated.length() != header.sealed_bytes {
        return Err(anyhow::anyhow!("journal identity changed during initialization").into());
    }
    entry.verify_current_unchanged(updated, max_disk_bytes)?;
    Ok((header, updated))
}

fn inspect_journal(
    entry: &PrivateFileEntry,
    file: &File,
    previous: FileIdentity,
    max_disk_bytes: u64,
) -> Result<FileIdentity, JournalError> {
    let current = FileIdentity::inspect_file_bounded(file, entry.path(), max_disk_bytes)?;
    if !previous.same_object(current) {
        return Err(anyhow::anyhow!("journal inode changed while opening").into());
    }
    entry.verify_current_unchanged(current, max_disk_bytes)?;
    Ok(current)
}

pub(super) fn truncate_uncommitted(file: &mut File, offset: u64) -> Result<(), JournalError> {
    file.set_len(offset)?;
    file.sync_all()?;
    Ok(())
}
