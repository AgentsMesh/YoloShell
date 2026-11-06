use super::constants::errors;
use super::intake::{CodexHookIntake, CodexIntakeOccurrence, CodexIntakeScan};
use super::intake_record::{CodexIntakeQuarantine, CodexIntakeRecord};
use anyhow::{Context, Result};

impl CodexHookIntake {
    pub fn scan(&self) -> Result<CodexIntakeScan> {
        let mut stream =
            crate::paths::directory_stream::DirectoryStream::open(self.directory().descriptor())
                .context(errors::INTAKE_STORAGE)?;
        let mut names = Vec::new();
        let mut entries_seen = 0_usize;
        while let Some(name) = stream.next_name().context(errors::INTAKE_STORAGE)? {
            entries_seen = entries_seen
                .checked_add(1)
                .context(errors::INTAKE_STORAGE)?;
            anyhow::ensure!(
                entries_seen <= super::intake_constants::MAX_DIRECTORY_ENTRIES,
                errors::INTAKE_STORAGE
            );
            if let Some(name) = name
                .to_str()
                .filter(|name| super::intake_record::is_pending_file(name))
            {
                names.push(name.to_owned());
            }
        }
        let mut scan = CodexIntakeScan::default();
        for name in names {
            self.scan_one(name, &mut scan);
        }
        scan.occurrences
            .sort_by_key(CodexIntakeOccurrence::ordering_key);
        Ok(scan)
    }

    fn scan_one(&self, name: String, scan: &mut CodexIntakeScan) {
        match self.is_quarantined(&name) {
            Ok(true) => {
                scan.quarantined = scan.quarantined.saturating_add(1);
                return;
            }
            Ok(false) => {}
            Err(_) => {
                scan.deferred = scan.deferred.saturating_add(1);
                return;
            }
        }
        let (bytes, identity) =
            match super::intake_storage::read_bytes(self.directory(), &self.path(&name)) {
                Ok(value) => value,
                Err(_) => {
                    scan.deferred = scan.deferred.saturating_add(1);
                    return;
                }
            };
        match CodexIntakeRecord::decode(&bytes, &name, self.owner()) {
            Ok(record) => scan
                .occurrences
                .push(CodexIntakeOccurrence::new(name, identity, record)),
            Err(_) => match self.write_quarantine(&name, identity) {
                Ok(()) => scan.quarantined = scan.quarantined.saturating_add(1),
                Err(_) => scan.deferred = scan.deferred.saturating_add(1),
            },
        }
    }

    fn is_quarantined(&self, pending_file: &str) -> Result<bool> {
        let marker = super::intake_record::quarantine_file_name(pending_file);
        let result = super::intake_storage::read_bytes(self.directory(), &self.path(&marker));
        match result {
            Ok((bytes, _)) => {
                let (_, pending_identity) =
                    super::intake_storage::read_bytes(self.directory(), &self.path(pending_file))?;
                CodexIntakeQuarantine::decode(&bytes, pending_file, pending_identity)?;
                Ok(true)
            }
            Err(error)
                if crate::paths::private_file::io_kind(&error)
                    == Some(std::io::ErrorKind::NotFound) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }
}
