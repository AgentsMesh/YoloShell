use super::constants::{
    LOCK_RECORD_MAX_BYTES, LOCK_RETRY, LOCK_TIMEOUT, MAX_SNAPSHOT_BYTES, SNAPSHOT_FILE_NAME,
    SNAPSHOT_LOCK_SUFFIX, errors,
};
use super::snapshot_authority::SnapshotAuthority;
use crate::paths::directory::RetainedPathVerification;
use crate::paths::identity::{FileIdentity, ObjectIdentity};
use crate::paths::private_file_lock::RetainedPrivateFileVerification;
use anyhow::{Context, Result, ensure};
use std::ffi::OsStr;
use std::fs::File;
use std::os::fd::{BorrowedFd, RawFd};
use std::path::{Path, PathBuf};

#[path = "snapshot_lock_failure.rs"]
mod failure;
#[path = "snapshot_lock_hooks.rs"]
mod hooks;

pub(super) struct SnapshotLock {
    directory: crate::paths::directory::PrivateDirectory,
    lock: crate::paths::private_file_lock::PrivateFileLock,
    target_identity: Option<FileIdentity>,
    authority: SnapshotAuthority,
}

impl SnapshotLock {
    pub(super) fn acquire(
        authority: &crate::paths::directory::PrivateDirectory,
        path: &Path,
    ) -> Result<Self> {
        validate_path(path)?;
        verify_directory(authority)?;
        let retained = Self::acquire_inner(authority, path)
            .map_err(|error| failure::classify_acquire(authority, error))?;
        retained.verify()?;
        hooks::run_after_acquire();
        Ok(retained)
    }

    pub(super) fn acquire_expected(
        directory: &crate::paths::directory::PrivateDirectory,
        path: &Path,
        expected: SnapshotAuthority,
    ) -> Result<Self> {
        let retained = match Self::acquire(directory, path) {
            Ok(retained) => retained,
            Err(error) if super::snapshot_error::is_indeterminate(&error) => {
                return Err(failure::classify_expected_acquire(
                    directory, path, expected, error,
                ));
            }
            Err(error) => return Err(error),
        };
        retained.authority.require(expected)?;
        Ok(retained)
    }

    fn acquire_inner(
        authority: &crate::paths::directory::PrivateDirectory,
        path: &Path,
    ) -> Result<Self> {
        let directory = authority.try_clone().context(errors::SNAPSHOT_LOCK)?;
        let lock = crate::paths::private_file_lock::PrivateFileLock::acquire_in_wait(
            directory.try_clone().context(errors::SNAPSHOT_LOCK)?,
            &lock_path(path),
            LOCK_RECORD_MAX_BYTES,
            LOCK_TIMEOUT,
            LOCK_RETRY,
        )
        .context(errors::SNAPSHOT_LOCK)?;
        hooks::run_before_authority_capture()?;
        let authority = SnapshotAuthority::new(
            directory.object_identity().context(errors::SNAPSHOT_LOCK)?,
            inspect_lock_identity(lock.descriptor())?,
        );
        let mut retained = Self {
            directory,
            lock,
            target_identity: None,
            authority,
        };
        retained.verify()?;
        let target_identity = retained
            .entry(path)
            .and_then(|entry| entry.inspect(MAX_SNAPSHOT_BYTES))
            .context(errors::SNAPSHOT_CORRUPT);
        retained.verify()?;
        retained.target_identity = target_identity?;
        Ok(retained)
    }

    pub(super) fn entry(
        &self,
        path: &Path,
    ) -> Result<crate::paths::private_file::PrivateFileEntry> {
        crate::paths::private_file::PrivateFileEntry::from_directory(
            self.directory.try_clone().context(errors::SNAPSHOT_LOCK)?,
            path,
        )
        .context(errors::SNAPSHOT_LOCK)
    }

    pub(super) fn verify(&self) -> Result<()> {
        self.verify_directory()?;
        match self.lock.verify_retained_path() {
            RetainedPrivateFileVerification::Verified => {}
            RetainedPrivateFileVerification::IdentityLost => {
                self.verify_directory()?;
                return Err(super::snapshot_error::lock_changed());
            }
            RetainedPrivateFileVerification::Indeterminate(error) => {
                self.verify_directory()?;
                return Err(super::snapshot_error::indeterminate(
                    error.context(errors::SNAPSHOT_LOCK),
                ));
            }
        }
        self.verify_directory()
    }

    pub(super) fn target_identity(&self) -> Option<FileIdentity> {
        self.target_identity
    }

    pub(super) fn authority(&self) -> SnapshotAuthority {
        self.authority
    }

    fn verify_directory(&self) -> Result<()> {
        verify_directory(&self.directory)
    }
}

fn inspect_lock_identity(descriptor: RawFd) -> Result<ObjectIdentity> {
    // The lock owns the raw descriptor for the duration of this borrow.
    let descriptor = unsafe { BorrowedFd::borrow_raw(descriptor) }
        .try_clone_to_owned()
        .context(errors::SNAPSHOT_LOCK)?;
    ObjectIdentity::from_file(&File::from(descriptor))
        .context(errors::SNAPSHOT_LOCK)?
        .context(errors::SNAPSHOT_LOCK)
}

fn verify_directory(directory: &crate::paths::directory::PrivateDirectory) -> Result<()> {
    match directory.verify_retained_path() {
        RetainedPathVerification::Verified => Ok(()),
        RetainedPathVerification::IdentityLost => Err(super::snapshot_error::directory_changed()),
        RetainedPathVerification::Indeterminate(error) => Err(
            super::snapshot_error::indeterminate(error.context(errors::SNAPSHOT_LOCK)),
        ),
    }
}

pub(super) fn validate_path(path: &Path) -> Result<()> {
    ensure!(
        path.is_absolute() && path.file_name() == Some(OsStr::new(SNAPSHOT_FILE_NAME)),
        errors::SNAPSHOT_PATH_INVALID
    );
    Ok(())
}

pub(super) fn lock_path(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(SNAPSHOT_LOCK_SUFFIX);
    PathBuf::from(value)
}

#[cfg(any(test, feature = "runtime-component-tests"))]
pub(super) use hooks::{set_after_acquire, set_before_authority_capture};
