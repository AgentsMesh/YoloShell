use crate::constants::message;
use crate::identity::{EvidenceIdentity, EvidenceKind};
use anyhow::{Context, Result};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::mem::MaybeUninit;
use std::os::fd::{FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

const OPEN_ERROR: libc::c_int = -1;

pub fn open_root(path: &Path) -> Result<Option<File>> {
    let name = CString::new(path.as_os_str().as_bytes()).context(message::DATA_HOME_NUL)?;
    open_descriptor(unsafe { libc::open(name.as_ptr(), directory_flags()) })
}

pub fn open_at(parent: RawFd, name: &CStr, flags: libc::c_int) -> Result<Option<File>> {
    open_descriptor(unsafe { libc::openat(parent, name.as_ptr(), flags) })
}

fn open_descriptor(descriptor: RawFd) -> Result<Option<File>> {
    if descriptor != OPEN_ERROR {
        return Ok(Some(unsafe { File::from_raw_fd(descriptor) }));
    }
    let error = std::io::Error::last_os_error();
    match error.raw_os_error() {
        Some(code) if [libc::ENOENT, libc::ELOOP, libc::ENOTDIR].contains(&code) => Ok(None),
        _ => Err(error).context(message::EVIDENCE_OPEN),
    }
}

pub fn entry_identity(
    parent: RawFd,
    name: &CStr,
    kind: EvidenceKind,
) -> Result<Option<EvidenceIdentity>> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            parent,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(EvidenceIdentity::from_stat(
            &unsafe { metadata.assume_init() },
            kind,
        ));
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(error).context(message::EVIDENCE_INSPECT)
    }
}

pub fn directory_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC
}

pub fn file_flags() -> libc::c_int {
    libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK
}
