use super::{OpenedPrivateFile, PRIVATE_FILE_MODE};
use crate::paths::identity::FileIdentity;
use crate::paths::process_file_lock::{Mode, ProcessFileLock};
use anyhow::{Context, Result};
use std::ffi::CStr;
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::path::Path;

pub(super) fn open_named(
    parent: RawFd,
    name: &CStr,
    path: &Path,
    flags: i32,
    max_bytes: u64,
) -> Result<OpenedPrivateFile> {
    let file = open_at(parent, name, flags)?;
    crate::private_acl::require_empty(&file, path)?;
    let identity = FileIdentity::inspect_file_bounded(&file, path, max_bytes)?;
    Ok(OpenedPrivateFile { file, identity })
}

pub(super) fn inspect_named(
    parent: RawFd,
    name: &CStr,
    path: &Path,
    max_bytes: u64,
) -> Result<FileIdentity> {
    open_named(parent, name, path, read_flags(), max_bytes).map(|opened| opened.identity)
}

pub(super) fn entry_stat(parent: RawFd, name: &CStr) -> std::io::Result<libc::stat> {
    let mut stat = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            stat.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        Ok(unsafe { stat.assume_init() })
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub(super) fn open_at(parent: RawFd, name: &CStr, flags: i32) -> std::io::Result<File> {
    let secure = flags | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK | libc::O_NOCTTY;
    let mode = libc::c_uint::try_from(PRIVATE_FILE_MODE).map_err(std::io::Error::other)?;
    let descriptor = unsafe { libc::openat(parent, name.as_ptr(), secure, mode) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

pub(super) fn set_private_mode(file: &File, path: &Path) -> Result<()> {
    let mode = libc::mode_t::try_from(PRIVATE_FILE_MODE)?;
    let result = unsafe { libc::fchmod(file.as_raw_fd(), mode) };
    if result != 0 {
        Err(std::io::Error::last_os_error().into())
    } else {
        crate::private_acl::clear(file, path)
    }
}

pub(super) fn lock_private_file(file: &File) -> Result<ProcessFileLock> {
    let identity = crate::paths::identity::ObjectIdentity::from_file(file)?
        .context("private file birth identity is unavailable")?;
    ProcessFileLock::try_lock(file, identity, Mode::Exclusive)?
        .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::WouldBlock))
        .context("lock private file")
}

pub(super) fn exchange_at(parent: RawFd, source: &CStr, target: &CStr) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_SWAP,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            parent,
            source.as_ptr(),
            parent,
            target.as_ptr(),
            libc::RENAME_EXCHANGE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub(super) fn rename_exclusive_at(
    parent: RawFd,
    source: &CStr,
    target: &CStr,
) -> std::io::Result<()> {
    rename_exclusive_between(parent, source, parent, target)
}

pub fn rename_exclusive_between(
    source_parent: RawFd,
    source: &CStr,
    target_parent: RawFd,
    target: &CStr,
) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            source_parent,
            source.as_ptr(),
            target_parent,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            source_parent,
            source.as_ptr(),
            target_parent,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

pub(super) fn read_flags() -> i32 {
    libc::O_RDONLY
}

pub(super) fn write_flags() -> i32 {
    libc::O_RDWR
}
