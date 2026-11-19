pub(super) use super::constants::ALIAS_MAIN_NAME;
use crate::paths::identity::ObjectIdentity;
use crate::paths::process_file_lock::{Mode, ProcessFileLock};
use anyhow::{Context, Result};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

pub(super) const ALIAS_ACTIVE_MODE: u32 = 0o700;
pub(super) const ALIAS_SEALED_MODE: u32 = 0o500;
const ALIAS_FILE_MODE: u32 = 0o600;
pub(super) const ALIAS_PREFIX: &str = ".yolo-sqlite-vfs-";
pub(super) const ALIAS_INITIALIZING_PREFIX: &str = ".yolo-sqlite-vfs-init-";
const ALIAS_JOURNAL_NAME: &str = "catalog-journal";
const ALIAS_WAL_NAME: &str = "catalog-wal";
const ALIAS_SHM_NAME: &str = "catalog-shm";
pub(super) const ALIAS_LOCK_NAME: &str = "owner.lock";
pub(super) const ALIAS_KNOWN_NAMES: [&str; 5] = [
    ALIAS_MAIN_NAME,
    ALIAS_JOURNAL_NAME,
    ALIAS_WAL_NAME,
    ALIAS_SHM_NAME,
    ALIAS_LOCK_NAME,
];
const EXPECTED_LINK_COUNT: u64 = 1;
const PERMISSION_MODE_MASK: u32 = 0o7777;

pub(super) struct AliasOwnerLock {
    _process_lock: ProcessFileLock,
    _file: File,
}

pub(super) fn create_lock_file(directory: &File, path: &Path) -> Result<AliasOwnerLock> {
    let flags = libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = openat(directory, ALIAS_LOCK_NAME, flags, ALIAS_FILE_MODE)?;
    let file = unsafe { File::from_raw_fd(fd) };
    let lock_path = path.join(ALIAS_LOCK_NAME);
    crate::private_acl::clear(&file, &lock_path)?;
    verify_private_file(&file, &lock_path)?;
    let identity = ObjectIdentity::from_file(&file)?
        .context("SQLite alias owner lock birth identity is unavailable")?;
    let process_lock = ProcessFileLock::try_lock(&file, identity, Mode::Exclusive)?
        .context("lock SQLite alias directory")?;
    #[cfg(all(test, feature = "yolo-unit-tests"))]
    super::secure_vfs_lock_hooks::run(path)?;
    anyhow::ensure!(
        lock_identity_matches(directory, path, identity)?,
        "SQLite alias owner lock identity changed"
    );
    super::secure_vfs_alias_directory::verify_path(directory, path)?;
    Ok(AliasOwnerLock {
        _file: file,
        _process_lock: process_lock,
    })
}

pub(super) fn open_lock_file(directory: &File, path: &Path) -> Result<Option<File>> {
    let flags = libc::O_RDWR | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = match openat(directory, ALIAS_LOCK_NAME, flags, ALIAS_FILE_MODE) {
        Ok(fd) => fd,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|error| error.kind() == std::io::ErrorKind::NotFound) =>
        {
            return Ok(None);
        }
        Err(error) => return Err(error),
    };
    let file = unsafe { File::from_raw_fd(fd) };
    verify_private_file(&file, &path.join(ALIAS_LOCK_NAME))?;
    Ok(Some(file))
}

pub(super) fn lock_identity_matches(
    directory: &File,
    path: &Path,
    expected: ObjectIdentity,
) -> Result<bool> {
    let Some(current) = open_lock_file(directory, path)? else {
        return Ok(false);
    };
    Ok(ObjectIdentity::from_file(&current)? == Some(expected))
}

pub(super) fn create_dummy_main(directory: &File, path: &Path) -> Result<()> {
    let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let fd = openat(directory, ALIAS_MAIN_NAME, flags, ALIAS_FILE_MODE)?;
    let file = unsafe { File::from_raw_fd(fd) };
    let path = path.join(ALIAS_MAIN_NAME);
    crate::private_acl::clear(&file, &path)?;
    verify_private_file(&file, &path)
}

pub(super) fn set_directory_mode(directory: &File, path: &Path, mode: u32) -> Result<()> {
    let native_mode = libc::mode_t::try_from(mode).context("SQLite alias mode is out of range")?;
    let result = unsafe { libc::fchmod(directory.as_raw_fd(), native_mode) };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("secure SQLite alias directory");
    }
    crate::private_acl::clear(directory, path)?;
    anyhow::ensure!(
        directory.metadata()?.mode() & PERMISSION_MODE_MASK == mode,
        "SQLite alias directory mode update failed"
    );
    Ok(())
}

pub(super) fn cleanup_directory(directory: &File) {
    for name in ALIAS_KNOWN_NAMES {
        unlink_name(directory, OsStr::new(name));
    }
}

pub(super) fn unlink_name(directory: &File, name: &OsStr) {
    if let Ok(name) = CString::new(name.as_bytes()) {
        let _ = unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
    }
}

pub(super) fn create_symlink(directory: &File, name: &OsStr, target: &Path) -> Result<()> {
    let name = CString::new(name.as_bytes())?;
    let target = CString::new(target.as_os_str().as_bytes())?;
    let result = unsafe { libc::symlinkat(target.as_ptr(), directory.as_raw_fd(), name.as_ptr()) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context("create SQLite alias reference")
    }
}

fn openat(directory: &File, name: &str, flags: libc::c_int, mode: u32) -> Result<libc::c_int> {
    let name = CString::new(name)?;
    let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags, mode) };
    if fd < 0 {
        Err(std::io::Error::last_os_error()).context("open SQLite alias state")
    } else {
        Ok(fd)
    }
}

fn verify_private_file(file: &File, path: &Path) -> Result<()> {
    crate::private_acl::require_empty(file, path)?;
    let metadata = file.metadata()?;
    anyhow::ensure!(metadata.is_file(), "SQLite alias state is not a file");
    anyhow::ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "SQLite alias file owner mismatch"
    );
    anyhow::ensure!(
        metadata.mode() & PERMISSION_MODE_MASK == ALIAS_FILE_MODE,
        "SQLite alias file mode mismatch"
    );
    anyhow::ensure!(
        metadata.nlink() == EXPECTED_LINK_COUNT,
        "SQLite alias file link count mismatch"
    );
    Ok(())
}
