use super::secure_vfs_alias_io::{ALIAS_ACTIVE_MODE, ALIAS_SEALED_MODE};
use anyhow::{Context, Result};
use std::ffi::{CString, OsStr};
use std::fs::{File, OpenOptions};
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::Path;

const PERMISSION_MODE_MASK: u32 = 0o7777;

enum DirectoryAcl {
    Clear,
    RequireEmpty,
}

pub(super) fn create(
    parent: &crate::paths::directory::PrivateDirectory,
    name: &OsStr,
    path: &Path,
) -> Result<File> {
    let name_c = CString::new(name.as_bytes())?;
    let mode = libc::mode_t::try_from(ALIAS_ACTIVE_MODE)?;
    let result = unsafe { libc::mkdirat(parent.descriptor(), name_c.as_ptr(), mode) };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("create SQLite alias directory");
    }
    open_at(parent, name, path, DirectoryAcl::Clear)
}

pub(super) fn open_verified(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    verify(&file, path, DirectoryAcl::RequireEmpty)?;
    Ok(file)
}

pub(super) fn open_verified_at(
    parent: &crate::paths::directory::PrivateDirectory,
    name: &OsStr,
    path: &Path,
) -> Result<File> {
    open_at(parent, name, path, DirectoryAcl::RequireEmpty)
}

pub(super) fn verify_path(directory: &File, path: &Path) -> Result<()> {
    let reopened = open_verified(path)?;
    verify_same(directory, &reopened, path)
}

pub(super) fn verify_entry(
    parent: &crate::paths::directory::PrivateDirectory,
    name: &OsStr,
    path: &Path,
    directory: &File,
) -> Result<()> {
    let reopened = open_verified_at(parent, name, path)?;
    verify_same(directory, &reopened, path)
}

fn open_at(
    parent: &crate::paths::directory::PrivateDirectory,
    name: &OsStr,
    path: &Path,
    acl: DirectoryAcl,
) -> Result<File> {
    let name = CString::new(name.as_bytes())?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::openat(parent.descriptor(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("open SQLite alias directory");
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    verify(&file, path, acl)?;
    Ok(file)
}

fn verify(file: &File, path: &Path, acl: DirectoryAcl) -> Result<()> {
    let metadata = file.metadata()?;
    anyhow::ensure!(metadata.is_dir(), "SQLite alias is not a directory");
    anyhow::ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "SQLite alias owner mismatch"
    );
    anyhow::ensure!(
        matches!(
            metadata.mode() & PERMISSION_MODE_MASK,
            ALIAS_ACTIVE_MODE | ALIAS_SEALED_MODE
        ),
        "SQLite alias mode mismatch"
    );
    match acl {
        DirectoryAcl::Clear => crate::private_acl::clear(file, path),
        DirectoryAcl::RequireEmpty => crate::private_acl::require_empty(file, path),
    }
}

fn verify_same(expected: &File, actual: &File, path: &Path) -> Result<()> {
    let expected = expected.metadata()?;
    let actual = actual.metadata()?;
    anyhow::ensure!(
        expected.dev() == actual.dev() && expected.ino() == actual.ino(),
        "SQLite alias directory identity changed: {}",
        path.display()
    );
    Ok(())
}
