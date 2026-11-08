use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::identity::DurableFileVersion;
use crate::paths::private_file::PrivateFileEntry;
use anyhow::{Context, Result, ensure};
use std::fs::File;
use std::os::fd::AsRawFd;
use uuid::Uuid;

const INVALID_MARKER: &str = "durable reboot publication marker is invalid";
const MARKER_READ_CONTEXT: &str = "read durable reboot publication marker";
#[cfg(test)]
const MARKER_REMOVE_CONTEXT: &str = "remove durable reboot publication marker";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
const MARKER_SYNC_CONTEXT: &str = "sync durable reboot publication marker";
#[cfg(any(test, feature = "adversarial-test-hooks"))]
const MARKER_WRITE_CONTEXT: &str = "write durable reboot publication marker";
const XATTR_NO_OPTIONS: libc::c_int = 0;
#[cfg(target_os = "macos")]
const XATTR_NO_POSITION: u32 = 0;
#[cfg(target_os = "macos")]
const PUBLICATION_MARKER_XATTR: &std::ffi::CStr = c"com.yolo.durable-reboot-transaction";
#[cfg(target_os = "linux")]
const PUBLICATION_MARKER_XATTR: &std::ffi::CStr = c"user.com.yolo.durable-reboot-transaction";

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn install(file: &File, transaction_id: Uuid) -> Result<()> {
    ensure!(!transaction_id.is_nil(), INVALID_MARKER);
    let value = transaction_id.as_bytes();
    let result = set_xattr(file, value);
    ensure_success(result, MARKER_WRITE_CONTEXT)?;
    file.sync_all().context(MARKER_SYNC_CONTEXT)?;
    verify(file, transaction_id)
}

pub(super) fn verify(file: &File, transaction_id: Uuid) -> Result<()> {
    ensure!(!transaction_id.is_nil(), INVALID_MARKER);
    let mut actual = Uuid::nil().into_bytes();
    let length = get_xattr(file, &mut actual);
    if length < 0 {
        return Err(std::io::Error::last_os_error()).context(MARKER_READ_CONTEXT);
    }
    ensure!(usize::try_from(length)? == actual.len(), INVALID_MARKER);
    ensure!(actual == transaction_id.into_bytes(), INVALID_MARKER);
    Ok(())
}

pub(super) fn verify_entry(
    entry: &PrivateFileEntry,
    expected: DurableFileVersion,
    transaction_id: Uuid,
) -> Result<()> {
    let opened = entry.open_read(MAX_DURABLE_JSON_BYTES)?;
    expected.verify_content_version_bounded(opened.file(), entry.path(), MAX_DURABLE_JSON_BYTES)?;
    verify(opened.file(), transaction_id)?;
    entry.verify_current_content_version(opened.identity(), MAX_DURABLE_JSON_BYTES)?;
    verify(opened.file(), transaction_id)
}

#[cfg(test)]
pub(super) fn remove(file: &File) -> Result<()> {
    let result = remove_xattr(file);
    ensure_success(result, MARKER_REMOVE_CONTEXT)?;
    file.sync_all().context(MARKER_SYNC_CONTEXT)
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
fn ensure_success(result: libc::c_int, context: &'static str) -> Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(context)
    }
}

#[cfg(target_os = "macos")]
#[cfg(any(test, feature = "adversarial-test-hooks"))]
fn set_xattr(file: &File, value: &[u8]) -> libc::c_int {
    unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            PUBLICATION_MARKER_XATTR.as_ptr(),
            value.as_ptr().cast(),
            value.len(),
            XATTR_NO_POSITION,
            XATTR_NO_OPTIONS,
        )
    }
}

#[cfg(target_os = "linux")]
#[cfg(any(test, feature = "adversarial-test-hooks"))]
fn set_xattr(file: &File, value: &[u8]) -> libc::c_int {
    unsafe {
        libc::fsetxattr(
            file.as_raw_fd(),
            PUBLICATION_MARKER_XATTR.as_ptr(),
            value.as_ptr().cast(),
            value.len(),
            XATTR_NO_OPTIONS,
        )
    }
}

#[cfg(target_os = "macos")]
fn get_xattr(file: &File, value: &mut [u8]) -> libc::ssize_t {
    unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            PUBLICATION_MARKER_XATTR.as_ptr(),
            value.as_mut_ptr().cast(),
            value.len(),
            XATTR_NO_POSITION,
            XATTR_NO_OPTIONS,
        )
    }
}

#[cfg(target_os = "linux")]
fn get_xattr(file: &File, value: &mut [u8]) -> libc::ssize_t {
    unsafe {
        libc::fgetxattr(
            file.as_raw_fd(),
            PUBLICATION_MARKER_XATTR.as_ptr(),
            value.as_mut_ptr().cast(),
            value.len(),
        )
    }
}

#[cfg(all(test, target_os = "macos"))]
fn remove_xattr(file: &File) -> libc::c_int {
    unsafe {
        libc::fremovexattr(
            file.as_raw_fd(),
            PUBLICATION_MARKER_XATTR.as_ptr(),
            XATTR_NO_OPTIONS,
        )
    }
}

#[cfg(all(test, target_os = "linux"))]
fn remove_xattr(file: &File) -> libc::c_int {
    unsafe { libc::fremovexattr(file.as_raw_fd(), PUBLICATION_MARKER_XATTR.as_ptr()) }
}
