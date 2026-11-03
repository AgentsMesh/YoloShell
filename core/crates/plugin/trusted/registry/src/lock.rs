use crate::{RegistryError, RegistryPaths};
use yolo_paths::coordination::CoordinationLock;

const LOCK_BYTES: u64 = 4096;
const ATTEMPTS: usize = 600;
const DELAY_MS: u64 = 10;

pub(crate) fn acquire(paths: &RegistryPaths) -> Result<CoordinationLock, RegistryError> {
    paths.ensure().map_err(|_| RegistryError::Paths)?;
    for _ in 0..ATTEMPTS {
        if let Some(lock) = CoordinationLock::try_acquire(&paths.lock_file(), LOCK_BYTES)
            .map_err(|_| RegistryError::Paths)?
        {
            return Ok(lock);
        }
        std::thread::sleep(std::time::Duration::from_millis(DELAY_MS));
    }
    Err(RegistryError::Busy)
}
