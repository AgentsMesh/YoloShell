use super::constants::{CURRENT_DIRECTORY_ENTRY, PARENT_DIRECTORY_ENTRY};
use anyhow::{Context, Result};
use std::ffi::{CStr, OsString};
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;

#[cfg(test)]
use std::cell::Cell;

const STREAM_OPEN_CONTEXT: &str = "open private directory stream";
const STREAM_READ_CONTEXT: &str = "read private directory stream";
#[cfg(test)]
const TEST_STREAM_OPEN_ERROR: libc::c_int = libc::EMFILE;

#[cfg(test)]
thread_local! {
    static FAIL_NEXT_STREAM_OPEN: Cell<bool> = const { Cell::new(false) };
    static FAIL_NEXT_STREAM_READ: Cell<bool> = const { Cell::new(false) };
}

pub struct DirectoryStream(*mut libc::DIR);

// A stream is moved into one maintenance thread and is never shared.
unsafe impl Send for DirectoryStream {}

impl DirectoryStream {
    pub fn open(descriptor: RawFd) -> Result<Self> {
        let current = std::ffi::CString::new(CURRENT_DIRECTORY_ENTRY)?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        let opened = unsafe { libc::openat(descriptor, current.as_ptr(), flags) };
        if opened < 0 {
            return Err(std::io::Error::last_os_error()).context(STREAM_OPEN_CONTEXT);
        }
        let stream = open_stream(opened);
        if stream.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe { libc::close(opened) };
            return Err(error).context(STREAM_OPEN_CONTEXT);
        }
        Ok(Self(stream))
    }

    pub fn next_name(&mut self) -> Result<Option<OsString>> {
        loop {
            set_errno(0);
            let entry = read_stream(self.0);
            if entry.is_null() {
                let code = errno();
                return if code == 0 {
                    Ok(None)
                } else {
                    Err(std::io::Error::from_raw_os_error(code)).context(STREAM_READ_CONTEXT)
                };
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name != CURRENT_DIRECTORY_ENTRY.as_bytes()
                && name != PARENT_DIRECTORY_ENTRY.as_bytes()
            {
                return Ok(Some(std::ffi::OsStr::from_bytes(name).to_owned()));
            }
        }
    }
}

fn open_stream(descriptor: RawFd) -> *mut libc::DIR {
    #[cfg(test)]
    if FAIL_NEXT_STREAM_OPEN.with(|failure| failure.replace(false)) {
        set_errno(TEST_STREAM_OPEN_ERROR);
        return std::ptr::null_mut();
    }
    unsafe { libc::fdopendir(descriptor) }
}

fn read_stream(stream: *mut libc::DIR) -> *mut libc::dirent {
    #[cfg(test)]
    if FAIL_NEXT_STREAM_READ.with(|failure| failure.replace(false)) {
        set_errno(libc::EIO);
        return std::ptr::null_mut();
    }
    unsafe { libc::readdir(stream) }
}

#[cfg(test)]
pub(crate) fn fail_next_stream_open_for_test() {
    FAIL_NEXT_STREAM_OPEN.with(|failure| failure.set(true));
}

#[cfg(test)]
fn fail_next_stream_read_for_test() {
    FAIL_NEXT_STREAM_READ.with(|failure| failure.set(true));
}

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
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

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/directory/directory_stream_fault_tests.rs"]
mod fault_tests;
