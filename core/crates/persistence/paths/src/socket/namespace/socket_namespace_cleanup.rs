use super::*;
use crate::paths::directory::entry_c_string;
use anyhow::{Context, Result};
use std::ffi::{CStr, OsStr};
use std::os::fd::RawFd;

const CLEANUP_OPEN_STREAM: &str = "open retained socket namespace directory stream";
const CLEANUP_READ_STREAM: &str = "read retained socket namespace directory stream";

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

pub(super) fn is_empty(directory: RawFd) -> Result<bool> {
    let mut empty = true;
    visit_entries(directory, |_| {
        empty = false;
        false
    })?;
    Ok(empty)
}

fn visit_entries<F>(directory: RawFd, mut visitor: F) -> Result<()>
where
    F: FnMut(&CStr) -> bool,
{
    let current = entry_c_string(OsStr::new(CURRENT_DIRECTORY_ENTRY))?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
    let descriptor = unsafe { libc::openat(directory, current.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context(CLEANUP_OPEN_STREAM);
    }
    let stream = unsafe { libc::fdopendir(descriptor) };
    if stream.is_null() {
        unsafe { libc::close(descriptor) };
        return Err(std::io::Error::last_os_error()).context(CLEANUP_OPEN_STREAM);
    }
    let stream = DirectoryStream(stream);
    loop {
        set_errno(0);
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            finish_readdir(errno())?;
            break;
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() != CURRENT_DIRECTORY_ENTRY.as_bytes()
            && name.to_bytes() != PARENT_DIRECTORY_ENTRY.as_bytes()
            && !visitor(name)
        {
            break;
        }
    }
    Ok(())
}

fn finish_readdir(error_code: libc::c_int) -> Result<()> {
    if error_code == 0 {
        Ok(())
    } else {
        Err(std::io::Error::from_raw_os_error(error_code)).context(CLEANUP_READ_STREAM)
    }
}

fn set_errno(value: libc::c_int) {
    unsafe { *errno_pointer() = value };
}

fn errno() -> libc::c_int {
    unsafe { *errno_pointer() }
}

fn errno_pointer() -> *mut libc::c_int {
    #[cfg(target_os = "macos")]
    unsafe {
        libc::__error()
    }
    #[cfg(not(target_os = "macos"))]
    unsafe {
        libc::__errno_location()
    }
}

pub(super) fn unlink(directory: RawFd, name: &CStr, flags: libc::c_int) -> std::io::Result<()> {
    let result = unsafe { libc::unlinkat(directory, name.as_ptr(), flags) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../../unit_tests/socket_namespace/socket_namespace_cleanup_inline_tests.rs"]
mod tests;
