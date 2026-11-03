use super::constants::{LOCK_RETRY, LOCK_TIMEOUT, MAX_OUTBOX_BYTES, OUTBOX_LOCK_SUFFIX, errors};
use crate::paths::identity::FileIdentity;
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};

pub(super) struct OutboxLock {
    directory: crate::paths::directory::PrivateDirectory,
    lock: crate::paths::private_file_lock::PrivateFileLock,
    target_identity: Option<FileIdentity>,
}

impl OutboxLock {
    pub(super) fn acquire(
        authority: &crate::paths::directory::PrivateDirectory,
        path: &Path,
    ) -> Result<Self> {
        let lock_path = lock_path(path);
        let directory = authority.try_clone().context(errors::OUTBOX_LOCK)?;
        let lock = crate::paths::private_file_lock::PrivateFileLock::acquire_in_wait(
            directory.try_clone().context(errors::OUTBOX_LOCK)?,
            &lock_path,
            super::constants::LOCK_RECORD_MAX_BYTES,
            LOCK_TIMEOUT,
            LOCK_RETRY,
        )
        .context(errors::OUTBOX_LOCK)?;
        Self::from_acquired(directory, lock, path)
    }

    pub(super) fn acquire_existing(
        authority: &crate::paths::directory::PrivateDirectory,
        path: &Path,
        expected_lock: FileIdentity,
    ) -> Result<Option<Self>> {
        let directory = authority.try_clone().context(errors::OUTBOX_LOCK)?;
        let lock = crate::paths::private_file_lock::PrivateFileLock::acquire_existing_in_wait(
            directory.try_clone().context(errors::OUTBOX_LOCK)?,
            &lock_path(path),
            super::constants::LOCK_RECORD_MAX_BYTES,
            LOCK_TIMEOUT,
            LOCK_RETRY,
        )
        .context(errors::OUTBOX_LOCK)?;
        let Some(lock) = lock else {
            return Ok(None);
        };
        ensure!(
            lock.identity().unchanged(expected_lock),
            errors::OUTBOX_LOCK
        );
        Self::from_acquired(directory, lock, path).map(Some)
    }

    fn from_acquired(
        directory: crate::paths::directory::PrivateDirectory,
        lock: crate::paths::private_file_lock::PrivateFileLock,
        path: &Path,
    ) -> Result<Self> {
        let target = crate::paths::private_file::PrivateFileEntry::from_directory(
            directory.try_clone().context(errors::OUTBOX_LOCK)?,
            path,
        )
        .context(errors::OUTBOX_CORRUPT)?;
        let target_identity = target
            .inspect(u64::try_from(MAX_OUTBOX_BYTES)?)
            .context(errors::OUTBOX_CORRUPT)?;
        let retained = Self {
            directory,
            lock,
            target_identity,
        };
        run_after_acquire_hook();
        Ok(retained)
    }

    pub(super) fn entry(
        &self,
        path: &Path,
    ) -> Result<crate::paths::private_file::PrivateFileEntry> {
        crate::paths::private_file::PrivateFileEntry::from_directory(
            self.directory.try_clone().context(errors::OUTBOX_LOCK)?,
            path,
        )
        .context(errors::OUTBOX_LOCK)
    }

    pub(super) fn verify(&self) -> Result<()> {
        self.directory
            .verify_descriptor_path()
            .context(errors::OUTBOX_LOCK)?;
        self.lock.verify().context(errors::OUTBOX_LOCK)?;
        self.directory
            .verify_descriptor_path()
            .context(errors::OUTBOX_LOCK)
    }

    pub(super) fn target_identity(&self) -> Option<FileIdentity> {
        self.target_identity
    }
}

pub(super) fn lock_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(OUTBOX_LOCK_SUFFIX);
    PathBuf::from(value)
}

pub(super) fn validate_path(path: &Path) -> Result<()> {
    ensure!(path.is_absolute(), errors::OUTBOX_LOCK);
    Ok(())
}

#[cfg(test)]
thread_local! {
    static AFTER_ACQUIRE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn set_after_acquire_hook(hook: impl FnOnce() + 'static) {
    AFTER_ACQUIRE_HOOK.with(|slot| *slot.borrow_mut() = Some(Box::new(hook)));
}

#[cfg(test)]
fn run_after_acquire_hook() {
    let hook = AFTER_ACQUIRE_HOOK.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

#[cfg(not(test))]
fn run_after_acquire_hook() {}
