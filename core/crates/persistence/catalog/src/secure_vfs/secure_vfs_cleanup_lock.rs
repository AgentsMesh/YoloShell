use super::secure_vfs_alias_directory::verify_entry;
use super::secure_vfs_alias_io::lock_identity_matches;
use crate::paths::directory::PrivateDirectory;
use crate::paths::identity::ObjectIdentity;
use crate::paths::process_file_lock::{Mode, ProcessFileLock};
use anyhow::{Context, Result};
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;

pub(super) struct RetainedAliasLock {
    _process_lock: ProcessFileLock,
    _file: File,
}

pub(super) fn retain_after_open(
    parent: &PrivateDirectory,
    name: &OsStr,
    path: &Path,
    directory: &File,
    file: File,
) -> Result<Option<RetainedAliasLock>> {
    let identity = ObjectIdentity::from_file(&file)?
        .context("SQLite alias cleanup lock birth identity is unavailable")?;
    let Some(process_lock) = ProcessFileLock::try_lock(&file, identity, Mode::Exclusive)? else {
        return Ok(None);
    };
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    super::secure_vfs_lock_hooks::run(path)?;
    if !verify_lock_binding(parent, name, path, directory, identity)? {
        return Ok(None);
    }
    Ok(Some(RetainedAliasLock {
        _process_lock: process_lock,
        _file: file,
    }))
}

fn verify_lock_binding(
    parent: &PrivateDirectory,
    name: &OsStr,
    path: &Path,
    directory: &File,
    identity: ObjectIdentity,
) -> Result<bool> {
    match lock_identity_matches(directory, path, identity) {
        Ok(true) => {}
        Ok(false) => return Ok(false),
        Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => return Ok(false),
        Err(error) => return Err(error),
    }
    match verify_entry(parent, name, path, directory) {
        Ok(()) => Ok(true),
        Err(error) if super::secure_vfs_cleanup_error::preserve(&error) => Ok(false),
        Err(error) => Err(error),
    }
}
