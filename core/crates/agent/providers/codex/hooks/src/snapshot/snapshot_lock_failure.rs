use super::*;

pub(super) fn classify_acquire(
    directory: &crate::paths::directory::PrivateDirectory,
    error: anyhow::Error,
) -> anyhow::Error {
    if crate::codex_hooks::snapshot_error::is_fence(&error) {
        return error;
    }
    if let Err(authority) = super::verify_directory(directory) {
        return authority;
    }
    crate::codex_hooks::snapshot_error::indeterminate(error.context(errors::SNAPSHOT_LOCK))
}

pub(super) fn classify_expected_acquire(
    directory: &crate::paths::directory::PrivateDirectory,
    path: &Path,
    expected: SnapshotAuthority,
    error: anyhow::Error,
) -> anyhow::Error {
    if let Err(authority) = super::verify_directory(directory) {
        return authority;
    }
    let observed = inspect_named_lock_identity(directory, path);
    if let Err(authority) = super::verify_directory(directory) {
        return authority;
    }
    match observed {
        Ok(observed) => expected.require_lock(observed).err().unwrap_or(error),
        Err(observation) => {
            crate::codex_hooks::snapshot_error::indeterminate(error.context(observation))
        }
    }
}

fn inspect_named_lock_identity(
    directory: &crate::paths::directory::PrivateDirectory,
    path: &Path,
) -> Result<Option<ObjectIdentity>> {
    let entry = crate::paths::private_file::PrivateFileEntry::from_directory(
        directory.try_clone().context(errors::SNAPSHOT_LOCK)?,
        &super::lock_path(path),
    )
    .context(errors::SNAPSHOT_LOCK)?;
    Ok(entry
        .inspect(LOCK_RECORD_MAX_BYTES)
        .context(errors::SNAPSHOT_LOCK)?
        .map(FileIdentity::object_identity))
}
