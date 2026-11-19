use super::*;

pub(in crate::codex_hooks) fn classify_commit_failure(
    lock: &SnapshotLock,
    path: &Path,
    expected: Option<FileIdentity>,
    committed: Option<FileIdentity>,
    error: anyhow::Error,
) -> anyhow::Error {
    if crate::codex_hooks::snapshot_error::is_fence(&error) {
        return error;
    }
    let current = match observe_current(lock, path) {
        Ok(current) => current,
        Err(authority) => return authority,
    };
    let externally_changed = match committed {
        Some(identity) => {
            current != expected
                && !current.is_some_and(|current| current.same_content_version(identity))
        }
        None => current != expected,
    };
    if externally_changed {
        crate::codex_hooks::snapshot_error::snapshot_changed()
    } else {
        crate::codex_hooks::snapshot_error::indeterminate(error.context(errors::SNAPSHOT_CORRUPT))
    }
}

pub(super) fn classify_read_failure(
    lock: &SnapshotLock,
    path: &Path,
    expected: Option<FileIdentity>,
    error: anyhow::Error,
) -> anyhow::Error {
    if crate::codex_hooks::snapshot_error::is_fence(&error) {
        return error;
    }
    match observe_current(lock, path) {
        Ok(current) if current != expected => {
            crate::codex_hooks::snapshot_error::snapshot_changed()
        }
        Ok(_) => error,
        Err(authority) => authority,
    }
}

pub(super) fn observe_current(lock: &SnapshotLock, path: &Path) -> Result<Option<FileIdentity>> {
    lock.verify()?;
    let current = lock
        .entry(path)?
        .inspect(MAX_SNAPSHOT_BYTES)
        .map_err(|error| {
            crate::codex_hooks::snapshot_error::indeterminate(
                error.context(errors::SNAPSHOT_CORRUPT),
            )
        })?;
    lock.verify()?;
    Ok(current)
}
