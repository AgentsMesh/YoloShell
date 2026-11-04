//! Package-local fault and fixture capabilities for persistence component tests.

#[path = "runtime_api.rs"]
mod runtime_api;
pub use runtime_api::*;

use crate::paths::directory::PrivateDirectory;
use crate::paths::socket_namespace::SocketNamespaceScanner;
use anyhow::Result;
use std::io::Write;
use std::os::fd::AsRawFd;

const FLOCK_SUCCESS: libc::c_int = 0;

pub struct SessionLifecycleLockGuard {
    _file: std::fs::File,
}

pub fn hold_session_lifecycle_lock(
    paths: &super::Paths,
    terminal_id: uuid::Uuid,
) -> Result<SessionLifecycleLockGuard> {
    let session = PrivateDirectory::open(&paths.session_dir(terminal_id))?;
    let path = session
        .path()
        .join(super::constants::SESSION_LIFECYCLE_LOCK_FILE_NAME);
    let entry = super::private_file::PrivateFileEntry::from_directory(session, &path)?;
    let opened = entry.open_rw_or_create(super::constants::SESSION_LIFECYCLE_LOCK_MAX_BYTES)?;
    let result = unsafe { libc::flock(opened.file().as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    anyhow::ensure!(result == FLOCK_SUCCESS, std::io::Error::last_os_error());
    entry.verify_current_unchanged(
        opened.identity(),
        super::constants::SESSION_LIFECYCLE_LOCK_MAX_BYTES,
    )?;
    let (file, _) = opened.into_parts();
    Ok(SessionLifecycleLockGuard { _file: file })
}

pub fn set_private_file_wait_contention_hook(hook: impl FnOnce() + 'static) {
    super::private_file_lock::PrivateFileLock::set_wait_contention_hook(hook);
}

pub fn fail_next_directory_sync() {
    crate::paths::directory::fail_next_sync();
}

pub fn install_recoverable_create_hook(hook: impl FnOnce() -> Result<()> + 'static) -> impl Drop {
    super::private_file::private_file_recoverable_create::install_recoverable_create_hook(hook)
}

pub fn set_before_private_create_hook(hook: impl FnOnce() + 'static) {
    super::private_file::private_file_missing::set_before_create_hook(hook);
}

pub fn set_before_empty_remove_hook(hook: impl FnOnce() + 'static) {
    super::directory::set_before_empty_remove_hook(hook);
}

pub fn try_acquire_existing_private_file_lock(
    path: &std::path::Path,
    max_bytes: u64,
) -> Result<Option<super::private_file_lock::PrivateFileLock>> {
    super::private_file_lock::PrivateFileLock::try_acquire_existing(path, max_bytes)
}

pub struct ProcessLockTurnGuard {
    _turn: super::process_file_lock::ProcessLockTurn,
}

pub fn try_acquire_process_lock_turn(
    identity: super::identity::ObjectIdentity,
) -> Result<Option<ProcessLockTurnGuard>> {
    Ok(
        super::process_file_lock::ProcessLockTurn::try_acquire(identity)?
            .map(|turn| ProcessLockTurnGuard { _turn: turn }),
    )
}

pub fn queued_process_lock_waiters(identity: super::identity::ObjectIdentity) -> usize {
    super::process_file_lock::ProcessLockTurn::queued_waiters_for_persistence_test(identity)
}

pub fn install_after_unlink_before_sync_hook<F>(
    entry: &super::private_file::PrivateFileEntry,
    hook: F,
) -> impl Drop + use<F>
where
    F: FnOnce() -> Result<()> + Send + 'static,
{
    entry.install_after_unlink_before_sync_hook(hook)
}

pub fn install_before_unlink_hook<F>(
    entry: &super::private_file::PrivateFileEntry,
    hook: F,
) -> impl Drop + use<F>
where
    F: FnOnce() -> Result<()> + 'static,
{
    entry.install_before_unlink_hook(hook)
}

pub fn install_after_move_sync_hook<F>(
    entry: &super::private_file::PrivateFileEntry,
    hook: F,
) -> impl Drop + use<F>
where
    F: FnOnce() -> Result<()> + 'static,
{
    entry.install_after_move_sync_hook(hook)
}

pub fn write_private_staging(
    entry: &super::private_file::PrivateFileEntry,
    bytes: &[u8],
    max_bytes: u64,
) -> Result<()> {
    let mut opened = entry.open_rw_or_create(max_bytes)?;
    opened.file_mut().write_all(bytes)?;
    opened.file().sync_all()?;
    entry.sync_parent()
}

pub fn unlink_owned_between_with_hook(
    lock: &mut super::private_file_lock::PrivateFileLock,
    active_name: &std::ffi::CStr,
    isolation_name: &std::ffi::CStr,
    hook: impl FnOnce() -> Result<()>,
) -> Result<()> {
    lock.unlink_owned_between_with_hook(active_name, isolation_name, hook)
}

pub const fn private_directory_mode() -> u32 {
    super::constants::PRIVATE_DIRECTORY_MODE
}

pub const fn retained_isolation_artifact_prefix() -> &'static str {
    super::constants::PRIVATE_FILE_RETAINED_ISOLATION_PREFIX
}

pub const fn retired_artifact_prefix() -> &'static str {
    super::constants::PRIVATE_FILE_RETIRED_PREFIX
}

pub fn retired_storage_artifact_name(target: &std::ffi::CStr) -> Result<std::ffi::CString> {
    super::private_file::private_file_names::retired_name_for_test(target)
}

pub fn scanner_for_override_root(root: &PrivateDirectory) -> Result<SocketNamespaceScanner> {
    SocketNamespaceScanner::from_root(root, true)
}

pub fn scanner_for_default_root(root: &PrivateDirectory) -> Result<SocketNamespaceScanner> {
    SocketNamespaceScanner::from_root(root, false)
}

pub fn set_retained_clone_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    crate::paths::private_file::set_retained_clone_hook(hook);
}

pub fn set_retained_cleanup_recycle_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    crate::paths::private_file::set_retained_cleanup_recycle_hook(hook);
}

pub fn set_retained_cleanup_sync_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    crate::paths::private_file::set_retained_cleanup_sync_hook(hook);
}

pub fn set_retained_exchange_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    crate::paths::private_file::set_retained_exchange_hook(hook);
}

pub fn set_retained_install_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    crate::paths::private_file::set_retained_install_hook(hook);
}

pub fn set_retained_rename_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    crate::paths::private_file::set_retained_rename_hook(hook);
}

pub fn set_retained_restore_hook(hook: impl FnOnce() -> Result<()> + 'static) {
    crate::paths::private_file::set_retained_restore_hook(hook);
}
