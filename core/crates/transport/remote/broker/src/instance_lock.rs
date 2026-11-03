use super::constants::BROKER_COORDINATION_LOCK_MAX_BYTES;
use crate::coordination::CoordinationLock;
use anyhow::{Context, Result};
use std::path::Path;

pub(super) struct BrokerInstanceLock {
    _lock: CoordinationLock,
}

impl BrokerInstanceLock {
    pub(super) fn acquire(path: &Path) -> Result<Self> {
        Self::try_acquire(path)?.with_context(|| format!("another broker owns {}", path.display()))
    }

    pub(super) fn try_acquire(path: &Path) -> Result<Option<Self>> {
        let acquired = CoordinationLock::try_acquire(path, BROKER_COORDINATION_LOCK_MAX_BYTES)?;
        acquired.map(Self::finish).transpose()
    }

    #[cfg(test)]
    pub(super) fn acquire_with_hook<F>(path: &Path, after_lock: F) -> Result<Self>
    where
        F: FnOnce() -> Result<()>,
    {
        let acquired = super::coordination_hook::try_acquire_with_hook(
            path,
            BROKER_COORDINATION_LOCK_MAX_BYTES,
            after_lock,
        )?;
        let lock = acquired.with_context(|| format!("another broker owns {}", path.display()))?;
        Self::finish(lock)
    }

    fn finish(mut lock: CoordinationLock) -> Result<Self> {
        let record = format!("{}\n", std::process::id());
        if let Err(error) = lock.replace_contents(record.as_bytes()) {
            drop(lock);
            return Err(error);
        }
        Ok(Self { _lock: lock })
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/coordination_replacement_tests.rs"]
mod tests;
