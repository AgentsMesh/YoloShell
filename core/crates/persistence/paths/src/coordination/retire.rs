use crate::paths::directory::PrivateDirectory;
use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::Result;

use super::lifecycle;

pub(super) fn reclaim(lock: &mut PrivateFileLock) -> Result<bool> {
    let parent = lock
        .path()
        .parent()
        .ok_or_else(|| anyhow::anyhow!(super::LOCK_PARENT_MISSING_ERROR))?;
    let root = PrivateDirectory::open(parent)?;
    let Some(_lifecycle) = lifecycle::LifecycleLock::try_acquire(&root)? else {
        return Ok(false);
    };
    reclaim_locked(lock, &root)
}

pub(super) fn reclaim_locked(lock: &mut PrivateFileLock, root: &PrivateDirectory) -> Result<bool> {
    anyhow::ensure!(
        lock.path().parent() == Some(root.path()),
        super::LOCK_PARENT_CHANGED_ERROR
    );
    root.verify_path()?;
    lock.unlink_owned()?;
    root.verify_path()?;
    Ok(true)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/retire_tests.rs"]
mod tests;
