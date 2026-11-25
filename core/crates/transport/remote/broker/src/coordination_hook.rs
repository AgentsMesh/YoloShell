use crate::coordination::CoordinationLock;
use anyhow::{Context, Result, ensure};
use std::os::unix::fs::MetadataExt;
use std::path::Path;

const LOCK_PATH_CHANGED_CONTEXT: &str = "verify locked private file path";

pub(super) fn try_acquire_with_hook<F>(
    path: &Path,
    max_bytes: u64,
    after_lock: F,
) -> Result<Option<CoordinationLock>>
where
    F: FnOnce() -> Result<()>,
{
    let Some(lock) = CoordinationLock::try_acquire(path, max_bytes)? else {
        return Ok(None);
    };
    let locked_identity = identity(path).context(LOCK_PATH_CHANGED_CONTEXT)?;
    after_lock()?;
    ensure!(
        identity(path).context(LOCK_PATH_CHANGED_CONTEXT)? == locked_identity,
        LOCK_PATH_CHANGED_CONTEXT
    );
    Ok(Some(lock))
}

fn identity(path: &Path) -> std::io::Result<(u64, u64)> {
    let metadata = std::fs::symlink_metadata(path)?;
    Ok((metadata.dev(), metadata.ino()))
}
