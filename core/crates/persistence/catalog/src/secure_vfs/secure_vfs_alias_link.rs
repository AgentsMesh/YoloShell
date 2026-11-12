use anyhow::{Context, Result};
use std::ffi::{CString, OsStr};
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStrExt;

pub(super) fn open(directory: &File, name: &OsStr) -> Result<File> {
    let name = CString::new(name.as_bytes())?;
    let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error()).context("open SQLite alias link")
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

pub(super) fn remove(directory: &File, name: &OsStr) -> Result<()> {
    let name = CString::new(name.as_bytes())?;
    let result = unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) };
    if result != 0 {
        return Err(std::io::Error::last_os_error()).context("remove SQLite alias link");
    }
    directory.sync_all().context("sync SQLite alias directory")
}
