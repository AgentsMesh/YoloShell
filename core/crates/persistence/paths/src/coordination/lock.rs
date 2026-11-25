use super::*;
use crate::paths::private_file_lock::PrivateFileLock;
use anyhow::Result;
use std::path::Path;

pub struct CoordinationLock {
    lock: PrivateFileLock,
    _lease: CoordinationLease,
}

impl CoordinationLock {
    pub fn verify(&self) -> Result<()> {
        self.lock.verify()
    }

    pub fn child_spawn_fence(&self) -> Result<CoordinationChildSpawnFence> {
        // A Darwin Mach receive right is task-local; only the file lock crosses fork.
        CoordinationChildSpawnFence::capture(&self.lock)
    }

    pub fn try_acquire(path: &Path, max_bytes: u64) -> Result<Option<Self>> {
        Self::try_acquire_inner(path, max_bytes, || Ok(()))
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
        Self::try_acquire_inner(path, max_bytes, after_lock)
    }

    fn try_acquire_inner<F>(path: &Path, max_bytes: u64, hook: F) -> Result<Option<Self>>
    where
        F: FnOnce() -> Result<()>,
    {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!(LOCK_PARENT_MISSING_ERROR))?;
        let Some(lease) = CoordinationLease::try_acquire(path)? else {
            return Ok(None);
        };
        crate::paths::directory::PrivateDirectory::ensure(parent)?;
        let lock = {
            #[cfg(any(test, feature = "persistence-test-support"))]
            {
                PrivateFileLock::try_acquire_with_hook(path, max_bytes, hook)
            }
            #[cfg(not(any(test, feature = "persistence-test-support")))]
            {
                let _ = hook;
                PrivateFileLock::try_acquire(path, max_bytes)
            }
        }?;
        Ok(lock.map(|lock| Self {
            lock,
            _lease: lease,
        }))
    }

    pub fn replace_contents(&mut self, bytes: &[u8]) -> Result<()> {
        self.lock.replace_contents(bytes)
    }
}

impl Drop for CoordinationLock {
    fn drop(&mut self) {
        if let Err(error) = retire::reclaim(&mut self.lock) {
            tracing::warn!(
                %error,
                lock = %self.lock.path().display(),
                message = LOCK_CLEANUP_FAILED
            );
        }
    }
}
