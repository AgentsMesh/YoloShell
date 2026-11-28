use super::directory::PrivateDirectory;
use super::identity::FileIdentity;
use super::private_file::{OpenedPrivateFile, PrivateFileEntry};
use super::process_file_lock::{Mode, ProcessFileLock, ProcessLockTurn};
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::fd::{AsRawFd, RawFd};
use std::path::Path;

#[path = "private_file_lock_cleanup.rs"]
mod cleanup;
#[path = "../private_file_unlink_proof.rs"]
mod unlink_proof;
#[path = "private_file_lock_verification.rs"]
mod verification;
#[path = "private_file_lock_wait.rs"]
mod wait;
pub use verification::RetainedPrivateFileVerification;

pub struct PrivateFileLock {
    entry: PrivateFileEntry,
    _process_lock: ProcessFileLock,
    _turn: ProcessLockTurn,
    file: File,
    identity: FileIdentity,
    max_bytes: u64,
}

impl PrivateFileLock {
    pub fn descriptor(&self) -> RawFd {
        self.file.as_raw_fd()
    }

    pub fn path(&self) -> &Path {
        self.entry.path()
    }

    pub(crate) fn try_acquire(path: &Path, max_bytes: u64) -> Result<Option<Self>> {
        Self::try_acquire_inner(path, max_bytes, true, || Ok(()))
    }

    #[cfg(any(test, feature = "persistence-test-support"))]
    pub(crate) fn try_acquire_existing(path: &Path, max_bytes: u64) -> Result<Option<Self>> {
        Self::try_acquire_inner(path, max_bytes, false, || Ok(()))
    }

    pub(crate) fn try_acquire_in(
        directory: PrivateDirectory,
        path: &Path,
        max_bytes: u64,
    ) -> Result<Option<Self>> {
        let entry = PrivateFileEntry::from_directory(directory, path)?;
        Self::try_acquire_entry(entry, max_bytes, true, || Ok(()))
    }

    pub fn try_acquire_existing_in(
        directory: PrivateDirectory,
        path: &Path,
        max_bytes: u64,
    ) -> Result<Option<Self>> {
        let entry = PrivateFileEntry::from_directory(directory, path)?;
        Self::try_acquire_entry(entry, max_bytes, false, || Ok(()))
    }

    #[cfg(any(test, feature = "persistence-test-support"))]
    pub(crate) fn try_acquire_with_hook<F>(
        path: &Path,
        max_bytes: u64,
        after_lock: F,
    ) -> Result<Option<Self>>
    where
        F: FnOnce() -> Result<()>,
    {
        Self::try_acquire_inner(path, max_bytes, true, after_lock)
    }

    fn try_acquire_inner<F>(
        path: &Path,
        max_bytes: u64,
        create: bool,
        after_lock: F,
    ) -> Result<Option<Self>>
    where
        F: FnOnce() -> Result<()>,
    {
        let entry = if create {
            PrivateFileEntry::ensure(path)?
        } else {
            PrivateFileEntry::open(path)?
        };
        Self::try_acquire_entry(entry, max_bytes, create, after_lock)
    }

    fn try_acquire_entry<F>(
        entry: PrivateFileEntry,
        max_bytes: u64,
        create: bool,
        after_lock: F,
    ) -> Result<Option<Self>>
    where
        F: FnOnce() -> Result<()>,
    {
        let opened = if create {
            entry.open_rw_or_create(max_bytes)
        } else {
            entry.open_rw(max_bytes)
        };
        let opened = match opened {
            Ok(opened) => opened,
            Err(error) if create && super::private_file::is_creation_contention(&error) => {
                return Ok(None);
            }
            Err(error)
                if !create
                    && super::private_file::io_kind(&error)
                        == Some(std::io::ErrorKind::NotFound) =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        let Some(turn) = ProcessLockTurn::try_acquire(opened.identity.object_identity())? else {
            return Ok(None);
        };
        let Some(process_lock) = ProcessFileLock::try_lock(
            &opened.file,
            opened.identity.object_identity(),
            Mode::Exclusive,
        )?
        else {
            return Ok(None);
        };
        after_lock()?;
        entry
            .verify_current_unchanged(opened.identity, max_bytes)
            .context("verify locked private file path")?;
        Ok(Some(Self {
            entry,
            _process_lock: process_lock,
            _turn: turn,
            file: opened.file,
            identity: opened.identity,
            max_bytes,
        }))
    }

    pub fn read_contents(&mut self) -> Result<Vec<u8>> {
        self.entry
            .verify_current_unchanged(self.identity, self.max_bytes)?;
        self.file.rewind().context("rewind private lock record")?;
        let mut bytes = Vec::new();
        std::io::Read::by_ref(&mut self.file)
            .take(self.max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .context("read private lock record")?;
        anyhow::ensure!(
            bytes.len() as u64 <= self.max_bytes,
            "lock record exceeds byte limit"
        );
        self.entry
            .verify_current_unchanged(self.identity, self.max_bytes)?;
        Ok(bytes)
    }

    pub(crate) fn replace_contents(&mut self, bytes: &[u8]) -> Result<()> {
        anyhow::ensure!(
            bytes.len() as u64 <= self.max_bytes,
            "lock record exceeds byte limit"
        );
        self.file
            .set_len(0)
            .context("truncate private lock record")?;
        self.file.rewind().context("rewind private lock record")?;
        self.file
            .write_all(bytes)
            .context("write private lock record")?;
        self.file.sync_data().context("sync private lock record")?;
        let identity =
            FileIdentity::inspect_file_bounded(&self.file, self.entry.path(), self.max_bytes)?;
        self.entry
            .verify_current_unchanged(identity, self.max_bytes)
            .context("verify published private lock record")?;
        self.identity = identity;
        Ok(())
    }
}
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/private_file/private_file_lock_first_create_process_tests.rs"]
mod first_create_process_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../../unit_tests/private/private_file_lock_instance_matrix.rs"]
mod instance_matrix;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/private_file/private_file_lock_process_wait_tests.rs"]
mod process_wait_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../../../../unit_tests/private/private_file_lock_wait_tests.rs"]
mod wait_tests;
