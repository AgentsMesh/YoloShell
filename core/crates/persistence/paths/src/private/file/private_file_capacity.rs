use super::private_file_cleanup_io::{lock_directory, try_lock_directory};
use super::{PrivateDirectoryLock, PrivateFileEntry};
use anyhow::{Context, Result};
use std::time::{Duration, Instant};

const PRIVATE_DIRECTORY_LOCK_TIMEOUT: &str = "private directory lock timed out";

impl PrivateFileEntry {
    pub fn lock_parent_exclusive(&self) -> Result<PrivateDirectoryLock> {
        self.directory.verify_path()?;
        let (file, process_lock) = lock_directory(self.directory.descriptor())?;
        let lock = PrivateDirectoryLock {
            _process_lock: process_lock,
            _file: file,
            _turn: None,
        };
        self.directory.verify_path()?;
        Ok(lock)
    }

    pub fn lock_parent_exclusive_wait(
        &self,
        timeout: Duration,
        retry_delay: Duration,
    ) -> Result<PrivateDirectoryLock> {
        self.directory.verify_path()?;
        let identity = self.directory.object_identity()?;
        let turn = crate::paths::process_file_lock::ProcessLockTurn::acquire(identity, timeout)?
            .context(PRIVATE_DIRECTORY_LOCK_TIMEOUT)?;
        let deadline = Instant::now() + timeout;
        loop {
            if let Some((file, process_lock)) = try_lock_directory(self.directory.descriptor())? {
                self.directory.verify_path()?;
                return Ok(PrivateDirectoryLock {
                    _process_lock: process_lock,
                    _file: file,
                    _turn: Some(turn),
                });
            }
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .context(PRIVATE_DIRECTORY_LOCK_TIMEOUT)?;
            std::thread::sleep(remaining.min(retry_delay));
        }
    }
}
