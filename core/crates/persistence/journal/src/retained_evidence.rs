use super::commit_cursor::read_cursor;
use super::header::read_header;
use super::scan::scan_committed;
use super::*;

const RETAINED_EVIDENCE_CURSOR_MISSING: &str = "retained journal commit cursor is missing";

pub struct RetainedJournalEvidence {
    entry: PrivateFileEntry,
    identity: FileIdentity,
    cursor_entry: PrivateFileEntry,
    cursor_identity: FileIdentity,
    last_sequence: u64,
}

impl Journal {
    pub fn inspect_retained_evidence(
        entry: PrivateFileEntry,
        runtime_epoch: Uuid,
    ) -> Result<RetainedJournalEvidence, JournalError> {
        let mut opened = entry.open_read(DEFAULT_MAX_DISK_BYTES)?;
        let identity = opened.identity();
        let header = read_header(opened.file_mut())?;
        let cursor_entry = entry.sibling_with_suffix(COMMIT_FILE_SUFFIX)?;
        let (cursor, cursor_identity) = read_cursor(&cursor_entry)?
            .ok_or_else(|| anyhow::anyhow!(RETAINED_EVIDENCE_CURSOR_MISSING))?;
        if cursor.generation != header.generation
            || cursor.committed_bytes < header.sealed_bytes
            || cursor.committed_bytes > identity.length()
        {
            return Err(JournalError::InvalidFormat);
        }
        let state = scan_committed(opened.file_mut(), cursor.committed_bytes, runtime_epoch)?;
        identity.verify_file_bounded(
            opened.file(),
            identity.length(),
            entry.path(),
            DEFAULT_MAX_DISK_BYTES,
        )?;
        let evidence = RetainedJournalEvidence {
            entry,
            identity,
            cursor_entry,
            cursor_identity,
            last_sequence: state.last_sequence,
        };
        evidence.verify_current()?;
        Ok(evidence)
    }
}

impl RetainedJournalEvidence {
    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn custody_retirement_evidence(
        &self,
        controller_generation: u64,
        runtime_file_identity: FileIdentity,
    ) -> crate::pty_custody::CustodyRetirementEvidence {
        crate::pty_custody::CustodyRetirementEvidence::new(
            controller_generation,
            runtime_file_identity,
            self.identity,
            self.cursor_identity,
            self.last_sequence,
        )
    }

    pub fn verify_current(&self) -> Result<(), JournalError> {
        self.entry
            .verify_current_unchanged(self.identity, DEFAULT_MAX_DISK_BYTES)?;
        self.cursor_entry
            .verify_current_unchanged(self.cursor_identity, RETAINED_CURSOR_BYTES as u64)?;
        Ok(())
    }
}
