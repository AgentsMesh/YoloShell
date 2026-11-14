use super::super::constants::{CURRENT_DIRECTORY_ENTRY, PARENT_DIRECTORY_ENTRY};
use anyhow::{Context, Result};
use std::ffi::{CStr, OsString};
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;

pub(super) struct DirectoryStream(*mut libc::DIR);

impl DirectoryStream {
    pub(super) fn open(descriptor: RawFd) -> Result<Self> {
        let current = std::ffi::CString::new(CURRENT_DIRECTORY_ENTRY)?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        let opened = unsafe { libc::openat(descriptor, current.as_ptr(), flags) };
        if opened < 0 {
            return Err(std::io::Error::last_os_error()).context("open private directory stream");
        }
        let stream = unsafe { libc::fdopendir(opened) };
        if stream.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe { libc::close(opened) };
            return Err(error).context("enumerate private directory");
        }
        Ok(Self(stream))
    }

    pub(super) fn next_name(&mut self) -> Result<Option<OsString>> {
        unsafe { *errno_location() = 0 };
        let entry = unsafe { libc::readdir(self.0) };
        if entry.is_null() {
            let code = unsafe { *errno_location() };
            return if code == 0 {
                Ok(None)
            } else {
                Err(std::io::Error::from_raw_os_error(code)).context("read private directory")
            };
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
        if name == CURRENT_DIRECTORY_ENTRY.as_bytes() || name == PARENT_DIRECTORY_ENTRY.as_bytes() {
            return self.next_name();
        }
        Ok(Some(std::ffi::OsStr::from_bytes(name).to_owned()))
    }
}

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

pub(super) fn lock_directory(
    descriptor: RawFd,
) -> Result<(
    std::fs::File,
    crate::paths::process_file_lock::ProcessFileLock,
)> {
    try_lock_directory(descriptor)?.context("lock private directory")
}

pub(super) fn try_lock_directory(
    descriptor: RawFd,
) -> Result<
    Option<(
        std::fs::File,
        crate::paths::process_file_lock::ProcessFileLock,
    )>,
> {
    let name = std::ffi::CString::new(CURRENT_DIRECTORY_ENTRY)?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let lock_descriptor = unsafe { libc::openat(descriptor, name.as_ptr(), flags) };
    if lock_descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("open private directory lock");
    }
    let file = unsafe { std::fs::File::from_raw_fd(lock_descriptor) };
    let identity = crate::paths::identity::ObjectIdentity::from_file(&file)?
        .context("private directory lock birth identity is unavailable")?;
    let Some(process_lock) = crate::paths::process_file_lock::ProcessFileLock::try_lock(
        &file,
        identity,
        crate::paths::process_file_lock::Mode::Exclusive,
    )?
    else {
        return Ok(None);
    };
    Ok(Some((file, process_lock)))
}

#[cfg(target_os = "macos")]
fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__error() }
}

#[cfg(target_os = "linux")]
fn errno_location() -> *mut libc::c_int {
    unsafe { libc::__errno_location() }
}
