use super::secure_vfs_alias_directory::verify_entry;
use super::secure_vfs_alias_io::{
    ALIAS_ACTIVE_MODE, ALIAS_KNOWN_NAMES, cleanup_directory, set_directory_mode,
};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const REMOVE_ENTRY_CONTEXT: &str = "remove stale SQLite alias entry";
const REMOVE_DIRECTORY_CONTEXT: &str = "remove stale SQLite alias directory";

pub(super) fn cleanup_best_effort(
    parent: &PrivateDirectory,
    name: &OsStr,
    path: &Path,
    directory: &File,
) {
    if verify_entry(parent, name, path, directory).is_err() {
        return;
    }
    let _ = set_directory_mode(directory, path, ALIAS_ACTIVE_MODE);
    cleanup_directory(directory);
    run_identity_hook(path);
    if verify_entry(parent, name, path, directory).is_ok()
        && remove_directory(parent, name).ok() == Some(true)
    {
        let _ = parent.sync();
    }
}

pub(super) fn cleanup_strict(
    parent: &PrivateDirectory,
    name: &OsStr,
    path: &Path,
    directory: &File,
) -> Result<bool> {
    if !verify_or_preserve(parent, name, path, directory)? {
        return Ok(false);
    }
    set_directory_mode(directory, path, ALIAS_ACTIVE_MODE)?;
    for child in ALIAS_KNOWN_NAMES {
        if !unlink_known(directory, child)? {
            return Ok(false);
        }
    }
    run_identity_hook(path);
    if !verify_or_preserve(parent, name, path, directory)? {
        return Ok(false);
    }
    run_io_hook(CleanupIoPoint::DirectoryRemove)?;
    if !remove_directory(parent, name)? {
        return Ok(false);
    }
    run_io_hook(CleanupIoPoint::ParentSync)?;
    parent.sync()?;
    Ok(true)
}

fn verify_or_preserve(
    parent: &PrivateDirectory,
    name: &OsStr,
    path: &Path,
    directory: &File,
) -> Result<bool> {
    match verify_entry(parent, name, path, directory) {
        Ok(()) => Ok(true),
        Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

fn unlink_known(directory: &File, name: &str) -> Result<bool> {
    use std::os::fd::AsRawFd;
    run_io_hook(CleanupIoPoint::EntryRemove)?;
    let name = CString::new(name)?;
    let result = unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
    if result == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(true)
    } else if super::secure_vfs_cleanup_error::preserve_io(&error) {
        Ok(false)
    } else {
        Err(error).context(REMOVE_ENTRY_CONTEXT)
    }
}

fn remove_directory(parent: &PrivateDirectory, name: &OsStr) -> Result<bool> {
    let name = CString::new(name.as_bytes())?;
    let result = unsafe { libc::unlinkat(parent.descriptor(), name.as_ptr(), libc::AT_REMOVEDIR) };
    if result == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if super::secure_vfs_cleanup_error::preserve_io(&error) {
        Ok(false)
    } else {
        Err(error).context(REMOVE_DIRECTORY_CONTEXT)
    }
}

fn run_identity_hook(path: &Path) {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    super::secure_vfs_alias_identity_tests::run_cleanup_hook(path);
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    let _ = path;
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum CleanupIoPoint {
    EntryRemove,
    DirectoryRemove,
    ParentSync,
}

fn run_io_hook(point: CleanupIoPoint) -> Result<()> {
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    return super::catalog_alias_maintenance_hooks::run_cleanup_io_hook(point);
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    let _ = point;
    #[cfg(not(all(test, feature = "yolo-unit-tests")))]
    Ok(())
}
