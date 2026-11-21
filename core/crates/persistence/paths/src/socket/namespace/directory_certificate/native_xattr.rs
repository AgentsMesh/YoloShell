use super::constants::*;
use anyhow::{Context, Result, ensure};
use std::os::fd::RawFd;

pub(super) fn read(descriptor: RawFd) -> Result<Option<Vec<u8>>> {
    let mut bytes = [u8::default(); ATTRIBUTE_MAX_BYTES];
    // SAFETY: the caller retains the fd, the name is NUL terminated, and the buffer is writable.
    #[cfg(target_os = "macos")]
    let count = unsafe {
        libc::fgetxattr(
            descriptor,
            ATTRIBUTE_NAME.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_POSITION,
            ATTRIBUTE_OPTIONS,
        )
    };
    #[cfg(target_os = "linux")]
    let count = unsafe {
        libc::fgetxattr(
            descriptor,
            ATTRIBUTE_NAME.as_ptr(),
            bytes.as_mut_ptr().cast(),
            bytes.len(),
        )
    };
    if count.is_negative() {
        let error = std::io::Error::last_os_error();
        if is_missing(&error) {
            return Ok(None);
        }
        return Err(error).context(READ_CONTEXT);
    }
    let count = usize::try_from(count)?;
    ensure!(count <= bytes.len(), INVALID_CERTIFICATE);
    Ok(Some(bytes[..count].to_vec()))
}

/// Create-only: EEXIST asks the caller to verify the current immutable certificate.
pub(super) fn create(descriptor: RawFd, bytes: &[u8]) -> Result<()> {
    ensure!(bytes.len() <= ATTRIBUTE_MAX_BYTES, INVALID_CERTIFICATE);
    // SAFETY: the caller retains the fd and the bytes/name live through this bounded syscall.
    #[cfg(target_os = "macos")]
    let result = unsafe {
        libc::fsetxattr(
            descriptor,
            ATTRIBUTE_NAME.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            ATTRIBUTE_POSITION,
            libc::XATTR_CREATE,
        )
    };
    #[cfg(target_os = "linux")]
    let result = unsafe {
        libc::fsetxattr(
            descriptor,
            ATTRIBUTE_NAME.as_ptr(),
            bytes.as_ptr().cast(),
            bytes.len(),
            libc::XATTR_CREATE,
        )
    };
    if result.is_negative() {
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(error).context(INSTALL_CONTEXT);
        }
    }
    Ok(())
}

fn is_missing(error: &std::io::Error) -> bool {
    #[cfg(target_os = "macos")]
    let missing = libc::ENOATTR;
    #[cfg(target_os = "linux")]
    let missing = libc::ENODATA;
    error.raw_os_error() == Some(missing)
}
