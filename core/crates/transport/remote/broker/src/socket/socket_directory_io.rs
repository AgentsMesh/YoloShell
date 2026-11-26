use anyhow::{Context, Result};
use std::ffi::CStr;
use std::os::fd::RawFd;

const REMOVE_QUARANTINED_SOCKET_CONTEXT: &str = "remove quarantined broker socket";
const RENAME_RETAINED_SOCKET_CONTEXT: &str = "rename retained broker socket";

pub(super) fn unlink(directory: RawFd, name: &CStr) -> Result<()> {
    let result = unsafe { libc::unlinkat(directory, name.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(REMOVE_QUARANTINED_SOCKET_CONTEXT)
    }
}

pub(super) fn rename_exclusive(
    source_directory: RawFd,
    source: &CStr,
    target_directory: RawFd,
    target: &CStr,
) -> Result<bool> {
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::renameatx_np(
            source_directory,
            source.as_ptr(),
            target_directory,
            target.as_ptr(),
            libc::RENAME_EXCL,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::renameat2(
            source_directory,
            source.as_ptr(),
            target_directory,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::AlreadyExists {
        Ok(false)
    } else {
        Err(error).context(RENAME_RETAINED_SOCKET_CONTEXT)
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_directory_io_tests.rs"]
mod tests;
