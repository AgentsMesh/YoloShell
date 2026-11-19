use anyhow::{Context, Result};
use std::ffi::{CStr, OsString};
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;

const OPEN_CONTEXT: &str = "open coordination directory stream";
const READ_CONTEXT: &str = "read coordination directory stream";
const CURRENT_DIRECTORY_ENTRY: &str = ".";
const PARENT_DIRECTORY_ENTRY: &str = "..";

pub(super) struct DirectoryStream(*mut libc::DIR);

impl DirectoryStream {
    pub(super) fn open(directory: RawFd) -> Result<Self> {
        Self::open_with(directory, |descriptor| unsafe {
            libc::fdopendir(descriptor)
        })
    }

    fn open_with<F>(directory: RawFd, into_stream: F) -> Result<Self>
    where
        F: FnOnce(RawFd) -> *mut libc::DIR,
    {
        let current = std::ffi::CString::new(CURRENT_DIRECTORY_ENTRY)?;
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        let descriptor = unsafe { libc::openat(directory, current.as_ptr(), flags) };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error()).context(OPEN_CONTEXT);
        }
        let stream = into_stream(descriptor);
        if stream.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe { libc::close(descriptor) };
            return Err(error).context(OPEN_CONTEXT);
        }
        Ok(Self(stream))
    }

    pub(super) fn next_name(&mut self) -> Result<Option<OsString>> {
        self.next_name_with(|stream| unsafe { libc::readdir(stream) })
    }

    fn next_name_with<F>(&mut self, mut read: F) -> Result<Option<OsString>>
    where
        F: FnMut(*mut libc::DIR) -> *mut libc::dirent,
    {
        loop {
            set_errno(0);
            let entry = read(self.0);
            if entry.is_null() {
                let code = errno();
                return if code == 0 {
                    Ok(None)
                } else {
                    Err(std::io::Error::from_raw_os_error(code)).context(READ_CONTEXT)
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

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/coordination/stream_tests.rs"]
mod tests;

// A stream is moved into one maintenance thread and is never shared.
unsafe impl Send for DirectoryStream {}

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
