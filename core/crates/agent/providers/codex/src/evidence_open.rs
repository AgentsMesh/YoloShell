use super::constants;
use super::evidence_identity::{EvidenceIdentity, EvidenceKind};
use anyhow::{Context, Result};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const OPEN_ERROR: libc::c_int = -1;

pub(super) fn open_root(path: &Path) -> Result<Option<File>> {
    let name = CString::new(path.as_os_str().as_bytes()).context(constants::DATA_HOME_NUL_ERROR)?;
    // SAFETY: name is NUL-terminated and flags require no variadic mode argument.
    open_descriptor(unsafe { libc::open(name.as_ptr(), directory_flags()) })
}

pub(super) fn open_at(parent: RawFd, name: &CStr, flags: libc::c_int) -> Result<Option<File>> {
    // SAFETY: parent is held open, name is NUL-terminated, and no mode argument is required.
    open_descriptor(unsafe { libc::openat(parent, name.as_ptr(), flags) })
}

fn open_descriptor(descriptor: RawFd) -> Result<Option<File>> {
    if descriptor != OPEN_ERROR {
        // SAFETY: a successful open/openat returns a newly owned descriptor.
        return Ok(Some(unsafe { File::from_raw_fd(descriptor) }));
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(code) if [libc::ENOENT, libc::ELOOP, libc::ENOTDIR].contains(&code) => Ok(None),
        _ => Err(error).context(constants::EVIDENCE_OPEN_ERROR),
    }
}

pub(super) fn entry_identity(
    parent: RawFd,
    name: &CStr,
    kind: EvidenceKind,
) -> Result<Option<EvidenceIdentity>> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    // SAFETY: metadata is writable and parent/name remain live for this call.
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        // SAFETY: fstatat initialized metadata after returning success.
        let metadata = unsafe { metadata.assume_init() };
        return Ok(EvidenceIdentity::from_stat(&metadata, kind));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(error).context(constants::EVIDENCE_INSPECT_ERROR)
    }
}

pub(super) fn directory_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC
}

pub(super) fn file_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK
}
