use super::*;
use std::ffi::CStr;
use std::fs::File;

const CLEANUP_IDENTITY_CHANGED: &str = "exclusive private file cleanup identity changed";
const CLEANUP_RETRY_ATTEMPTS: usize = 2;
const CLEANUP_RETRIES_EXHAUSTED: &str = "exclusive private file cleanup exhausted retries";
const CLEANUP_SYNC_CONTEXT: &str = "sync exclusive private file";
const CLEANUP_TRUNCATE_CONTEXT: &str = "truncate exclusive private file";

impl PrivateFileEntry {
    pub(super) fn discard_owned_name(
        &self,
        name: &CStr,
        file: &File,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<bool> {
        let descriptor = FileIdentity::inspect_file_bounded(file, &self.path, max_bytes)?;
        anyhow::ensure!(descriptor.same_object(expected), CLEANUP_IDENTITY_CHANGED);
        if !self.owned_name_is_present(name, descriptor, max_bytes)? {
            return Ok(false);
        }
        file.set_len(0).context(CLEANUP_TRUNCATE_CONTEXT)?;
        file.sync_all().context(CLEANUP_SYNC_CONTEXT)?;
        let truncated = FileIdentity::inspect_file_bounded(file, &self.path, max_bytes)?;
        anyhow::ensure!(truncated.same_object(expected), CLEANUP_IDENTITY_CHANGED);
        let mut failure = None;
        for _ in 0..CLEANUP_RETRY_ATTEMPTS {
            match self.discard_named_identity(name, truncated, max_bytes) {
                Ok(()) => return Ok(true),
                Err(error) => failure = Some(error),
            }
        }
        Err(failure.context(CLEANUP_RETRIES_EXHAUSTED)?)
    }

    fn owned_name_is_present(
        &self,
        name: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<bool> {
        match open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            read_flags(),
            max_bytes,
        ) {
            Ok(opened) => {
                anyhow::ensure!(
                    opened.identity.same_object(expected),
                    CLEANUP_IDENTITY_CHANGED
                );
                Ok(true)
            }
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
                self.unlink_isolation_matches(name, expected, max_bytes)
            }
            Err(error) => Err(error),
        }
    }

    fn discard_named_identity(
        &self,
        name: &CStr,
        expected: FileIdentity,
        max_bytes: u64,
    ) -> Result<()> {
        if entry_is_missing(&self.directory, name)?
            && !self.unlink_isolation_matches(name, expected, max_bytes)?
        {
            return Ok(());
        }
        self.unlink_named_identity(name, expected, max_bytes, || Ok(()))
    }
}
