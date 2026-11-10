use crate::constants::{
    DIRECTORY_LOCK_ERROR, DIRECTORY_LOCK_IDENTITY_ERROR, DIRECTORY_LOCK_OPEN_ERROR,
    DIRECTORY_LOCK_SUCCESS, RETAINED_DIRECTORY_SELF_NAME,
};
use crate::directory::{DirectoryHandle, directory_flags};
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;

const OPEN_ERROR: libc::c_int = -1;

#[derive(Debug)]
pub struct DirectoryLock {
    _file: File,
}

pub(crate) fn lock_exclusive(directory: &DirectoryHandle) -> Result<DirectoryLock> {
    directory.verify_self()?;
    let name = CString::new(RETAINED_DIRECTORY_SELF_NAME)?;
    let descriptor = unsafe { libc::openat(directory.raw_fd(), name.as_ptr(), directory_flags()) };
    if descriptor == OPEN_ERROR {
        return Err(std::io::Error::last_os_error()).context(DIRECTORY_LOCK_OPEN_ERROR);
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    let metadata = file.metadata()?;
    let identity = directory.identity();
    ensure!(
        metadata.dev() == identity.device && metadata.ino() == identity.inode,
        DIRECTORY_LOCK_IDENTITY_ERROR
    );
    let status = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if status != DIRECTORY_LOCK_SUCCESS {
        return Err(std::io::Error::last_os_error()).context(DIRECTORY_LOCK_ERROR);
    }
    directory.verify_self()?;
    Ok(DirectoryLock { _file: file })
}
