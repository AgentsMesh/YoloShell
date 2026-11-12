use super::names;
use anyhow::{Context, Result, ensure};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const CURRENT_DIRECTORY_ENTRY: &[u8] = b".";
const PARENT_DIRECTORY_ENTRY: &[u8] = b"..";
const CURRENT_DIRECTORY_NAME: &CStr = c".";

pub(super) fn ensure_empty(directory: &File) -> Result<()> {
    visit_entries(directory.as_raw_fd(), |name| {
        anyhow::bail!(
            "broker generation staging slot contains unknown entry: {}",
            name.to_string_lossy()
        )
    })
}

pub(super) fn ensure_known(directory: &File, path: &Path) -> Result<()> {
    visit_entries(directory.as_raw_fd(), |name| {
        ensure!(
            names::is_generation_file_name(name),
            "broker generation contains unknown entry: {}",
            name.to_string_lossy()
        );
        verify_known_file(directory, path, name)
    })
}

fn verify_known_file(directory: &File, path: &Path, name: &CStr) -> Result<()> {
    let flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    let descriptor = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context("open broker generation file");
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    let path = path.join(std::ffi::OsStr::from_bytes(name.to_bytes()));
    crate::private_acl::require_empty(&file, &path)
}

fn visit_entries<F>(descriptor: RawFd, mut visitor: F) -> Result<()>
where
    F: FnMut(&CStr) -> Result<()>,
{
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let reopened = unsafe { libc::openat(descriptor, CURRENT_DIRECTORY_NAME.as_ptr(), flags) };
    if reopened < 0 {
        return Err(std::io::Error::last_os_error()).context("reopen generation directory");
    }
    let stream = unsafe { libc::fdopendir(reopened) };
    if stream.is_null() {
        unsafe { libc::close(reopened) };
        return Err(std::io::Error::last_os_error()).context("open generation directory stream");
    }
    let stream = DirectoryStream(stream);
    loop {
        set_errno(0);
        let entry = unsafe { libc::readdir(stream.0) };
        if entry.is_null() {
            let error = errno();
            ensure!(
                error == 0,
                "read generation directory failed: OS error {error}"
            );
            return Ok(());
        }
        let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) };
        if name.to_bytes() != CURRENT_DIRECTORY_ENTRY && name.to_bytes() != PARENT_DIRECTORY_ENTRY {
            visitor(name)?;
        }
    }
}

struct DirectoryStream(*mut libc::DIR);

impl Drop for DirectoryStream {
    fn drop(&mut self) {
        unsafe { libc::closedir(self.0) };
    }
}

#[cfg(target_os = "macos")]
fn errno() -> libc::c_int {
    unsafe { *libc::__error() }
}

#[cfg(target_os = "macos")]
fn set_errno(value: libc::c_int) {
    unsafe { *libc::__error() = value };
}

#[cfg(target_os = "linux")]
fn errno() -> libc::c_int {
    unsafe { *libc::__errno_location() }
}

#[cfg(target_os = "linux")]
fn set_errno(value: libc::c_int) {
    unsafe { *libc::__errno_location() = value };
}
