use super::*;

const STAGED_CREATE_CLEANUP_CONTEXT: &str = "clean failed staged private file creation";
const STAGED_CREATE_RECOVERY_CONTEXT: &str = "recover staged private file creation";
const CANONICAL_UNLINK_RECOVERY_CONTEXT: &str = "recover canonical private unlink isolation";

impl PrivateFileEntry {
    pub fn open_read(&self, max_bytes: u64) -> Result<OpenedPrivateFile> {
        self.open_stable(read_flags(), max_bytes)
    }

    pub fn open_rw_or_create(&self, max_bytes: u64) -> Result<OpenedPrivateFile> {
        self.recover_canonical_unlink_isolation(max_bytes)?;
        self.recover_staged_creation_isolation(max_bytes)?;
        if entry_is_missing(&self.directory, &self.name)? {
            return self.open_missing_staged(max_bytes);
        }
        self.open_stable(write_flags(), max_bytes)
    }

    fn recover_canonical_unlink_isolation(&self, max_bytes: u64) -> Result<()> {
        let isolation = self.unlink_isolation_entry()?;
        if isolation.is_missing()? {
            return Ok(());
        }
        let Some(mut lock) =
            crate::paths::private_file_lock::PrivateFileLock::try_acquire_existing_in(
                self.directory.try_clone()?,
                isolation.path(),
                max_bytes,
            )?
        else {
            return Err(std::io::Error::from(std::io::ErrorKind::WouldBlock))
                .context(CANONICAL_UNLINK_RECOVERY_CONTEXT);
        };
        lock.reclaim_unlink_isolation_of(self)
            .context(CANONICAL_UNLINK_RECOVERY_CONTEXT)
    }

    fn recover_staged_creation_isolation(&self, max_bytes: u64) -> Result<()> {
        let active_name = super::private_file_names::active_name(
            &self.name,
            crate::paths::constants::ATOMIC_FILE_PREFIX,
            crate::paths::constants::ATOMIC_FILE_SUFFIX,
        )?;
        let active = self.sibling_named(&active_name)?;
        let isolation = active.unlink_isolation_entry()?;
        if isolation.is_missing()? {
            return Ok(());
        }
        let Some(mut lock) =
            crate::paths::private_file_lock::PrivateFileLock::try_acquire_existing_in(
                self.directory.try_clone()?,
                isolation.path(),
                max_bytes,
            )?
        else {
            return Err(std::io::Error::from(std::io::ErrorKind::WouldBlock))
                .context(STAGED_CREATE_RECOVERY_CONTEXT);
        };
        lock.reclaim_isolation_preserving_source(&active)
            .context(STAGED_CREATE_RECOVERY_CONTEXT)
    }

    fn open_missing_staged(&self, max_bytes: u64) -> Result<OpenedPrivateFile> {
        private_file_missing::run_before_create_hook();
        let mut temporary = self.temporary_single_attempt(
            crate::paths::constants::ATOMIC_FILE_PREFIX,
            crate::paths::constants::ATOMIC_FILE_SUFFIX,
        )?;
        if temporary.previous.is_some() {
            return self.open_staged_creation_winner(&temporary, max_bytes);
        }
        temporary.sync_and_inspect(&self.path, max_bytes)?;
        if !entry_is_missing(&self.directory, &self.name)? {
            return self.open_staged_creation_winner(&temporary, max_bytes);
        }
        match self.rename_temporary(&temporary, max_bytes) {
            Ok(()) => self.open_stable(write_flags(), max_bytes),
            Err(error)
                if crate::paths::private_file::io_kind(&error)
                    == Some(std::io::ErrorKind::AlreadyExists) =>
            {
                self.open_staged_creation_winner(&temporary, max_bytes)
            }
            Err(error) => {
                let target_present =
                    matches!(entry_is_missing(&self.directory, &self.name), Ok(false));
                let stage_retained = self
                    .active_identity_matches(&temporary.name, temporary.identity)
                    .unwrap_or(false);
                if target_present && stage_retained {
                    return self.open_staged_creation_winner(&temporary, max_bytes);
                }
                if let Err(cleanup) = self.discard_staged_creation(&temporary, max_bytes) {
                    return Err(
                        error.context(format!("{STAGED_CREATE_CLEANUP_CONTEXT}: {cleanup:#}"))
                    );
                }
                Err(error)
            }
        }
    }

    fn open_staged_creation_winner(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        anyhow::ensure!(
            self.discard_staged_creation(temporary, max_bytes)?,
            STAGED_CREATE_CLEANUP_CONTEXT
        );
        self.open_stable(write_flags(), max_bytes)
    }

    fn discard_staged_creation(
        &self,
        temporary: &TemporaryPrivateFile,
        max_bytes: u64,
    ) -> Result<bool> {
        self.discard_owned_name(
            &temporary.name,
            &temporary.file,
            temporary.identity,
            max_bytes,
        )
    }

    pub(crate) fn open_rw(&self, max_bytes: u64) -> Result<OpenedPrivateFile> {
        self.open_stable(write_flags(), max_bytes)
    }

    pub(super) fn open_stable(
        &self,
        flags: libc::c_int,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        let opened = self.open_once(flags, max_bytes)?;
        run_after_open_hook();
        self.verify_current_unchanged(opened.identity, max_bytes)?;
        Ok(opened)
    }

    pub(super) fn open_once(
        &self,
        flags: libc::c_int,
        max_bytes: u64,
    ) -> Result<OpenedPrivateFile> {
        open_named(
            self.directory.descriptor(),
            &self.name,
            &self.path,
            flags,
            max_bytes,
        )
    }
}

#[cfg(test)]
thread_local! {
    static AFTER_OPEN_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> = const {
        std::cell::RefCell::new(None)
    };
}

#[cfg(test)]
pub(crate) fn set_after_open_hook(hook: impl FnOnce() + 'static) {
    AFTER_OPEN_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_after_open_hook() {
    AFTER_OPEN_HOOK.with(|slot| {
        if let Some(hook) = slot.borrow_mut().take() {
            hook();
        }
    });
}

#[cfg(not(test))]
fn run_after_open_hook() {}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../../unit_tests/private/private_file_open_tests.rs"]
mod tests;
