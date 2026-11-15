use super::commit_cursor_format::{CursorRecord, decode_cursor, encode_cursor};
use super::*;

#[path = "commit_cursor_retained.rs"]
mod retained;
#[path = "commit_cursor_retained_recovery.rs"]
mod retained_recovery;
#[path = "commit_cursor_retained_stage.rs"]
mod retained_stage;
#[path = "commit_cursor_slots.rs"]
mod slots;

#[derive(Debug)]
pub(super) struct CommitCursor {
    entry: PrivateFileEntry,
    identity: FileIdentity,
    pub(super) record: CursorRecord,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::journal) struct CommitCursorEvidence {
    pub(in crate::journal) identity: FileIdentity,
    pub record: CursorRecord,
}

#[derive(Debug)]
pub(super) enum LoadedCursor {
    Current(CommitCursor),
    Repair(CursorRepair),
}

#[derive(Debug)]
pub(super) struct CursorRepair {
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    record: CursorRecord,
}

impl LoadedCursor {
    pub(super) fn load(
        entry: PrivateFileEntry,
        header: JournalHeader,
        file_len: u64,
    ) -> Result<Self, JournalError> {
        let _lock = entry.lock_and_cleanup(COMMIT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
        let loaded = match read_cursor(&entry)? {
            Some(cursor) => {
                retained_recovery::settle_retired_cursor(&entry, cursor.0)?;
                Some(cursor)
            }
            None => retained_recovery::restore_retired_cursor(&entry, header, file_len)?,
        };
        match loaded {
            Some((record, identity)) if record.generation == header.generation => {
                if record.committed_bytes < header.sealed_bytes || record.committed_bytes > file_len
                {
                    return Err(JournalError::InvalidFormat);
                }
                Ok(Self::Current(CommitCursor {
                    entry,
                    identity,
                    record,
                }))
            }
            Some((_, identity)) if file_len == header.sealed_bytes => Ok(Self::Repair(
                CursorRepair::new(entry, Some(identity), header),
            )),
            None if file_len == header.sealed_bytes => {
                Ok(Self::Repair(CursorRepair::new(entry, None, header)))
            }
            _ => Err(JournalError::InvalidFormat),
        }
    }

    pub(super) fn committed_bytes(&self) -> u64 {
        match self {
            Self::Current(cursor) => cursor.record.committed_bytes,
            Self::Repair(repair) => repair.record.committed_bytes,
        }
    }

    pub(super) fn requires_repair(&self) -> bool {
        matches!(self, Self::Repair(_))
    }

    pub(super) fn finish(self) -> Result<CommitCursor, JournalError> {
        match self {
            Self::Current(cursor) => Ok(cursor),
            Self::Repair(repair) => repair.finish(),
        }
    }
}

impl CursorRepair {
    fn new(entry: PrivateFileEntry, expected: Option<FileIdentity>, header: JournalHeader) -> Self {
        Self {
            entry,
            expected,
            record: CursorRecord {
                generation: header.generation,
                committed_bytes: header.sealed_bytes,
                publication_revision: INITIAL_CURSOR_REVISION,
            },
        }
    }

    fn finish(self) -> Result<CommitCursor, JournalError> {
        install_cursor(self.entry, self.expected, self.record)
    }
}

impl CommitCursor {
    pub(in crate::journal) fn evidence(&self) -> CommitCursorEvidence {
        CommitCursorEvidence {
            identity: self.identity,
            record: self.record,
        }
    }

    pub(super) fn publish(&mut self, committed_bytes: u64) -> Result<(), JournalError> {
        let record = CursorRecord {
            generation: self.record.generation,
            committed_bytes,
            publication_revision: self
                .record
                .publication_revision
                .checked_add(CURSOR_REVISION_INCREMENT)
                .ok_or(JournalError::CursorRevisionExhausted)?,
        };
        let entry = self.entry.reopen()?;
        let replacement = install_cursor(entry, Some(self.identity), record)?;
        *self = replacement;
        Ok(())
    }

    pub(super) fn verify_current(&self) -> Result<(), JournalError> {
        self.entry
            .verify_current_unchanged(self.identity, RETAINED_CURSOR_BYTES as u64)?;
        Ok(())
    }
}

fn install_cursor(
    entry: PrivateFileEntry,
    expected: Option<FileIdentity>,
    record: CursorRecord,
) -> Result<CommitCursor, JournalError> {
    let _lock = entry.lock_and_cleanup(COMMIT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
    let mut temporary = entry.temporary(COMMIT_TEMPORARY_PREFIX, TEMPORARY_SUFFIX)?;
    let result = (|| {
        temporary.file_mut().write_all(&encode_cursor(record))?;
        temporary.sync_and_inspect(entry.path(), COMMIT_CURSOR_BYTES as u64)?;
        entry.verify_expected(expected, RETAINED_CURSOR_BYTES as u64)?;
        entry.rename_temporary(&temporary, RETAINED_CURSOR_BYTES as u64)?;
        entry.verify_current_content_version(temporary.identity(), RETAINED_CURSOR_BYTES as u64)?;
        let (installed, identity) = read_cursor(&entry)?.ok_or(JournalError::InvalidFormat)?;
        if installed != record || !identity.same_content_version(temporary.identity()) {
            return Err(JournalError::InvalidFormat);
        }
        Ok(identity)
    })();
    if result.is_err() {
        entry.remove_temporary(&temporary)?;
    }
    result.map(|identity| CommitCursor {
        entry,
        identity,
        record,
    })
}

pub(in crate::journal) fn read_cursor(
    entry: &PrivateFileEntry,
) -> Result<Option<(CursorRecord, FileIdentity)>, JournalError> {
    let Some(_) = entry.inspect(RETAINED_CURSOR_BYTES as u64)? else {
        return Ok(None);
    };
    let opened = entry.open_read(RETAINED_CURSOR_BYTES as u64)?;
    let identity = opened.identity();
    let record = slots::read_active_cursor(opened.file(), identity, entry.path())?;
    entry.verify_current_unchanged(identity, RETAINED_CURSOR_BYTES as u64)?;
    Ok(Some((record, identity)))
}

pub(super) fn cursor_exists(entry: &PrivateFileEntry) -> Result<bool, JournalError> {
    Ok(entry.inspect(RETAINED_CURSOR_BYTES as u64)?.is_some())
}
