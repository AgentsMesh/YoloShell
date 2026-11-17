use anyhow::{Context, Result};
use std::ffi::{CStr, OsString};
use std::os::fd::RawFd;
use std::os::unix::ffi::OsStrExt;

const OPEN_CONTEXT: &str = "open catalog alias directory stream";
const READ_CONTEXT: &str = "read catalog alias directory stream";
const CURRENT_ENTRY: &[u8] = b".";
const PARENT_ENTRY: &[u8] = b"..";

pub(super) struct AliasDirectoryStream(*mut libc::DIR);

impl AliasDirectoryStream {
    pub(super) fn open(directory: RawFd) -> Result<Self> {
        let name = c".";
        let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
        let descriptor = unsafe { libc::openat(directory, name.as_ptr(), flags) };
        if descriptor < 0 {
            return Err(std::io::Error::last_os_error()).context(OPEN_CONTEXT);
        }
        let stream = unsafe { libc::fdopendir(descriptor) };
        if stream.is_null() {
            let error = std::io::Error::last_os_error();
            unsafe { libc::close(descriptor) };
            return Err(error).context(OPEN_CONTEXT);
        }
        Ok(Self(stream))
    }

    pub(super) fn next_name(&mut self) -> Result<Option<OsString>> {
        loop {
            set_errno(0);
            let entry = unsafe { libc::readdir(self.0) };
            if entry.is_null() {
                let code = errno();
                return if code == 0 {
                    Ok(None)
                } else {
                    Err(std::io::Error::from_raw_os_error(code)).context(READ_CONTEXT)
                };
            }
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name != CURRENT_ENTRY && name != PARENT_ENTRY {
                return Ok(Some(std::ffi::OsStr::from_bytes(name).to_owned()));
            }
        }
    }
}

// The stream is moved into exactly one maintenance thread and is never shared.
unsafe impl Send for AliasDirectoryStream {}

impl Drop for AliasDirectoryStream {
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
