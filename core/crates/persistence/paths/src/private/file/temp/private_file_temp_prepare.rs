use super::private_file_names::{quarantine_name, retired_name, staging_name};
use super::*;
use crate::paths::constants::PRIVATE_FILE_TEMPORARY_RETRY_MILLIS;
use std::time::Duration;

const UNSYNCED_STAGE_CLEANUP_CONTEXT: &str = "clean unsynced private staging file";
const STAGING_SLOT_LOCKED_ERROR: &str = "all private staging slots are retained";

impl PrivateFileEntry {
    pub(super) fn prepare_active<F, G, H>(
        &self,
        active: &CString,
        slot: &CString,
        after_open: &mut F,
        before_truncate: &mut G,
        before_create: &mut H,
    ) -> Result<Option<(File, crate::paths::process_file_lock::ProcessFileLock)>>
    where
        F: FnMut() -> Result<()>,
        G: FnMut() -> Result<()>,
        H: FnMut() -> Result<()>,
    {
        let slot_file = match open_named(
            self.directory.descriptor(),
            slot,
            &self.path,
            write_flags(),
            u64::MAX,
        ) {
            Ok(opened) => Some(opened),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => None,
            Err(error) => return Err(error),
        };
        if let Some(opened) = slot_file {
            after_open()?;
            let Some(process_lock) = lock_or_retry(&opened.file)? else {
                return Ok(None);
            };
            before_truncate()?;
            if !self.active_identity_matches(slot, opened.identity)? {
                return Ok(None);
            }
            self.truncate_staging(&opened.file)?;
            let identity = FileIdentity::inspect_file_bounded(&opened.file, &self.path, u64::MAX)?;
            self.move_named_identity(slot, active, identity, u64::MAX)?;
            return Ok(Some((opened.file, process_lock)));
        }
        let opened = match self.open_or_create_active(active, before_create) {
            Ok(opened) => opened,
            Err(error) if retryable(&error) => return Ok(None),
            Err(error) => return Err(error),
        };
        after_open()?;
        let Some(process_lock) = lock_or_retry(&opened.file)? else {
            return Ok(None);
        };
        before_truncate()?;
        if !self.active_identity_matches(active, opened.identity)? {
            return Ok(None);
        }
        self.truncate_staging(&opened.file)?;
        Ok(Some((opened.file, process_lock)))
    }

    fn truncate_staging(&self, file: &File) -> Result<()> {
        file.set_len(0).context("truncate private staging file")?;
        file.sync_all()
            .context("sync truncated private staging file")
    }

    pub(super) fn active_identity_matches(
        &self,
        name: &CString,
        expected: FileIdentity,
    ) -> Result<bool> {
        match open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            read_flags(),
            u64::MAX,
        ) {
            Ok(current) => Ok(current.identity.unchanged(expected)),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn open_or_create_active<H>(
        &self,
        active: &CString,
        before_create: &mut H,
    ) -> Result<OpenedPrivateFile>
    where
        H: FnMut() -> Result<()>,
    {
        match open_named(
            self.directory.descriptor(),
            active,
            &self.path,
            write_flags(),
            u64::MAX,
        ) {
            Ok(opened) => Ok(opened),
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
                before_create()?;
                let flags = write_flags() | libc::O_CREAT | libc::O_EXCL;
                let file = open_at(self.directory.descriptor(), active, flags)?;
                set_private_mode(&file, &self.path)?;
                let identity = FileIdentity::inspect_file_bounded(&file, &self.path, u64::MAX)?;
                if let Err(error) = self.directory.sync() {
                    return match self.discard_owned_name(active, &file, identity, u64::MAX) {
                        Ok(_) => Err(error),
                        Err(cleanup) => {
                            Err(error
                                .context(format!("{UNSYNCED_STAGE_CLEANUP_CONTEXT}: {cleanup:#}")))
                        }
                    };
                }
                Ok(OpenedPrivateFile { file, identity })
            }
            Err(error) => Err(error),
        }
    }

    pub(super) fn available_staging_slot(&self, active: &CString, prefix: &str) -> Result<CString> {
        let retired = retired_name(&self.name)?;
        let quarantine = quarantine_name(&self.name, prefix, active)?;
        for candidate in [&retired, &quarantine] {
            if !entry_is_missing(&self.directory, candidate)?
                && self.staging_slot_available(candidate)?
            {
                return Ok(candidate.clone());
            }
        }
        let staging = staging_name(&self.name, prefix)?;
        if self.staging_slot_available(&staging)? {
            return Ok(staging);
        }
        for candidate in [retired, quarantine] {
            if self.staging_slot_available(&candidate)? {
                return Ok(candidate);
            }
        }
        Err(std::io::Error::from(std::io::ErrorKind::WouldBlock)).context(STAGING_SLOT_LOCKED_ERROR)
    }

    fn staging_slot_available(&self, name: &CString) -> Result<bool> {
        if entry_is_missing(&self.directory, name)? {
            return Ok(true);
        }
        let opened = match open_named(
            self.directory.descriptor(),
            name,
            &self.path,
            write_flags(),
            u64::MAX,
        ) {
            Ok(opened) => opened,
            Err(error) if io_kind(&error) == Some(std::io::ErrorKind::NotFound) => {
                return Ok(true);
            }
            Err(error) => return Err(error),
        };
        Ok(lock_or_retry(&opened.file)?.is_some())
    }
}

pub(super) fn lock_or_retry(
    file: &File,
) -> Result<Option<crate::paths::process_file_lock::ProcessFileLock>> {
    match lock_private_file(file) {
        Ok(process_lock) => Ok(Some(process_lock)),
        Err(error) if retryable(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(super) fn retryable(error: &anyhow::Error) -> bool {
    matches!(
        io_kind(error),
        Some(std::io::ErrorKind::WouldBlock | std::io::ErrorKind::AlreadyExists)
    )
}

pub(super) fn retry_temporary() -> Result<()> {
    #[cfg(test)]
    super::private_file_temp_hooks::run_before_retry()?;
    std::thread::sleep(Duration::from_millis(PRIVATE_FILE_TEMPORARY_RETRY_MILLIS));
    Ok(())
}
