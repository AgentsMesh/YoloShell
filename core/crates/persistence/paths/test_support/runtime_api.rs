use crate::coordination::CoordinationLock;
use crate::paths::private_file::{PrivateDirectoryLock, PrivateFileEntry};
use crate::paths::socket_activation::SocketPublication;
use anyhow::Result;
use std::path::Path;

impl crate::Paths {
    pub fn socket_root_override_for_test(&self) -> Option<&Path> {
        self.socket_root_override()
    }
}

pub const fn staging_artifact_prefix() -> &'static str {
    crate::paths::constants::PRIVATE_FILE_STAGING_PREFIX
}

pub fn remove_private_file_with_hook(
    entry: &PrivateFileEntry,
    hook: impl FnMut() -> Result<()>,
) -> Result<()> {
    entry.remove_with_hook(hook)
}

pub fn acquire_coordination_with_hook<F>(
    path: &Path,
    max_bytes: u64,
    hook: F,
) -> Result<Option<CoordinationLock>>
where
    F: FnOnce() -> Result<()>,
{
    CoordinationLock::try_acquire_with_hook(path, max_bytes, hook)
}

pub fn cleanup_private_files_with_hook<F>(
    entry: &PrivateFileEntry,
    prefix: &str,
    suffix: &str,
    max_entries: usize,
    hook: F,
) -> Result<PrivateDirectoryLock>
where
    F: FnMut(&Path) -> Result<()>,
{
    entry.lock_and_cleanup_bounded_with(prefix, suffix, max_entries, hook)
}

pub fn set_socket_before_unlink<F>(publication: &mut SocketPublication, hook: F)
where
    F: FnOnce() + Send + Sync + 'static,
{
    publication.set_before_unlink(hook);
}

pub fn queued_session_lifecycle_waiters(
    paths: &crate::Paths,
    terminal_id: uuid::Uuid,
) -> Result<usize> {
    let directory =
        crate::paths::directory::PrivateDirectory::open(&paths.session_dir(terminal_id))?;
    let path = directory
        .path()
        .join(crate::paths::constants::SESSION_LIFECYCLE_LOCK_FILE_NAME);
    let entry = PrivateFileEntry::from_directory(directory, &path)?;
    let limit = crate::paths::constants::SESSION_LIFECYCLE_LOCK_MAX_BYTES;
    let opened = entry.open_rw(limit)?;
    let queued = super::queued_process_lock_waiters(opened.identity().object_identity());
    entry.verify_current_unchanged(opened.identity(), limit)?;
    Ok(queued)
}
