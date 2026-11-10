use super::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::Result;

pub struct LifecycleLock {
    _lock: PrivateFileLock,
    _lease: CoordinationLease,
}

impl LifecycleLock {
    pub(super) fn try_acquire(root: &PrivateDirectory) -> Result<Option<Self>> {
        root.verify_path()?;
        let path = root.path().join(LIFECYCLE_LOCK_FILE);
        let Some(lease) = CoordinationLease::try_acquire(&path)? else {
            return Ok(None);
        };
        let Some(lock) =
            PrivateFileLock::try_acquire_in(root.try_clone()?, &path, SWEEP_LOCK_MAX_BYTES)?
        else {
            return Ok(None);
        };
        root.verify_path()?;
        Ok(Some(Self {
            _lock: lock,
            _lease: lease,
        }))
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/lifecycle_tests.rs"]
mod tests;
