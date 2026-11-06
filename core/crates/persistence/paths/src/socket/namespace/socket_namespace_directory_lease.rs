use crate::paths::{
    directory::PrivateDirectory,
    identity::ObjectIdentity,
    process_file_lock::{Mode, ProcessFileLock},
};
use anyhow::{Context, Result, ensure};
use std::{fs::File, os::fd::BorrowedFd};

const PARTICIPANT_BUSY: &str = "socket namespace directory ownership is busy";
const DIRECTORY_CHANGED: &str = "socket namespace directory lease identity changed";

/// All admitted participants retain shared ownership of the actual transport inode.
/// Exclusive ownership permits only certified, empty transport directory reclamation.
#[derive(Debug)]
pub(super) struct DirectoryLease {
    _lock: ProcessFileLock,
    identity: ObjectIdentity,
}

impl DirectoryLease {
    pub(super) fn shared(directory: &PrivateDirectory) -> Result<Self> {
        Self::try_lock(directory, Mode::Shared)?.context(PARTICIPANT_BUSY)
    }

    pub(super) fn try_exclusive(directory: &PrivateDirectory) -> Result<Option<Self>> {
        Self::try_lock(directory, Mode::Exclusive)
    }

    fn try_lock(directory: &PrivateDirectory, mode: Mode) -> Result<Option<Self>> {
        directory.verify_path()?;
        let identity = directory.object_identity()?;
        // SAFETY: the retained directory keeps this descriptor valid while duplicating it.
        let descriptor =
            unsafe { BorrowedFd::borrow_raw(directory.descriptor()) }.try_clone_to_owned()?;
        let file = File::from(descriptor);
        let Some(lock) = ProcessFileLock::try_lock(&file, identity, mode)? else {
            return Ok(None);
        };
        let lease = Self {
            _lock: lock,
            identity,
        };
        lease.verify(directory)?;
        Ok(Some(lease))
    }

    pub(super) fn verify(&self, directory: &PrivateDirectory) -> Result<()> {
        directory.verify_path()?;
        ensure!(
            directory.object_identity()? == self.identity,
            DIRECTORY_CHANGED
        );
        Ok(())
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_directory_lease_tests.rs"]
mod tests;
