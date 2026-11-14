use super::constants::errors;
use super::{CodexHookIntake, CodexIntakeOccurrence};
use crate::agent::hooks::{EventSequence, HookEnvelope};
use anyhow::{Result, ensure};

pub(super) enum CodexReceiptArtifact {
    Active(HookEnvelope, super::CodexOccurrenceIdentity),
    Isolated(HookEnvelope, super::CodexOccurrenceIdentity),
    Missing,
}

impl CodexHookIntake {
    pub fn find_exact_occurrence(
        &self,
        expected: &HookEnvelope,
    ) -> Result<Option<CodexIntakeOccurrence>> {
        ensure!(
            expected.identity == *self.owner()
                && !expected.event_id.is_nil()
                && expected.sequence == EventSequence::ZERO,
            errors::INTAKE_CORRUPT
        );
        let scan = self.scan()?;
        ensure!(scan.deferred == usize::default(), errors::INTAKE_STORAGE);
        let mut matches = scan
            .occurrences
            .into_iter()
            .filter(|occurrence| occurrence.envelope().event_id == expected.event_id);
        let Some(occurrence) = matches.next() else {
            return Ok(None);
        };
        ensure!(matches.next().is_none(), errors::INTAKE_CORRUPT);
        ensure!(occurrence.envelope() == expected, errors::INTAKE_CORRUPT);
        self.verify_current(&occurrence)?;
        Ok(Some(occurrence))
    }

    pub(super) fn read_receipt_artifact(&self, file_name: &str) -> Result<CodexReceiptArtifact> {
        let path = self.path(file_name);
        if let Some(artifact) = self.read_receipt_path(&path, file_name)? {
            return Ok(CodexReceiptArtifact::Active(artifact.0, artifact.1));
        }
        let source = crate::paths::private_file::PrivateFileEntry::from_directory(
            self.directory().try_clone()?,
            &path,
        )?;
        let isolation = source.unlink_isolation_entry()?;
        if let Some(artifact) = self.read_receipt_path(isolation.path(), file_name)? {
            return Ok(CodexReceiptArtifact::Isolated(artifact.0, artifact.1));
        }
        self.directory()
            .verify_path()
            .map_err(|_| anyhow::anyhow!(errors::INTAKE_STORAGE))?;
        Ok(CodexReceiptArtifact::Missing)
    }

    fn read_receipt_path(
        &self,
        path: &std::path::Path,
        file_name: &str,
    ) -> Result<Option<(HookEnvelope, super::CodexOccurrenceIdentity)>> {
        let (bytes, file_identity) = match super::intake_storage::read_bytes(self.directory(), path)
        {
            Ok(value) => value,
            Err(error)
                if crate::paths::private_file::io_kind(&error)
                    == Some(std::io::ErrorKind::NotFound) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let record =
            super::intake_record::CodexIntakeRecord::decode(&bytes, file_name, self.owner())?;
        let envelope = record.envelope().clone();
        let identity =
            super::CodexOccurrenceIdentity::capture(file_name, file_identity, &envelope)?;
        Ok(Some((envelope, identity)))
    }
}
