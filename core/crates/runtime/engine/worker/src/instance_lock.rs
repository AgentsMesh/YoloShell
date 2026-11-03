use super::constants::{WORKER_INSTANCE_BUSY_CONTEXT, WORKER_INSTANCE_LOCK_MAX_BYTES};
use crate::coordination::CoordinationLock;
use crate::paths::Paths;
use anyhow::{Result, bail};
use std::path::Path;
use uuid::Uuid;

pub(super) struct WorkerInstanceLock {
    _lock: CoordinationLock,
}

impl WorkerInstanceLock {
    pub(super) fn acquire(paths: &Paths, terminal_id: Uuid) -> Result<Self> {
        let session = terminal_id.to_string();
        let path = crate::coordination::lock_path(
            crate::coordination::CoordinationScope::Worker,
            paths.home(),
            Some(&session),
        )?;
        Self::acquire_path(&path)
    }

    fn acquire_path(path: &Path) -> Result<Self> {
        let Some(lock) = CoordinationLock::try_acquire(path, WORKER_INSTANCE_LOCK_MAX_BYTES)?
        else {
            bail!("{WORKER_INSTANCE_BUSY_CONTEXT}: {}", path.display());
        };
        Ok(Self { _lock: lock })
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../unit_tests/instance_lock_tests.rs"]
mod tests;
