use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::fd::FromRawFd;
use std::os::unix::fs::FileExt;
use yolo_release_contracts::{
    ARCHIVE_NAME, ARTIFACT_CHANGED_ERROR, MAX_ARTIFACT_BYTES, PRIVATE_FILE_MODE,
};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::DIGEST_BUFFER_SIZE;

const CREATE_ERROR: &str = "create pinned candidate artifact";
const REOPEN_ERROR: &str = "reopen pinned candidate artifact";

pub fn create(root: &DirectoryHandle) -> Result<File> {
    let name = CString::new(ARCHIVE_NAME)?;
    let descriptor = unsafe {
        libc::openat(
            root.raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            PRIVATE_FILE_MODE as libc::c_uint,
        )
    };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context(CREATE_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub fn reopen(root: &DirectoryHandle) -> Result<File> {
    let name = CString::new(ARCHIVE_NAME)?;
    let descriptor = unsafe {
        libc::openat(
            root.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor < 0 {
        return Err(std::io::Error::last_os_error()).context(REOPEN_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub fn copy(source: &File, destination: &mut File, length: u64) -> Result<()> {
    ensure!(length <= MAX_ARTIFACT_BYTES, ARTIFACT_CHANGED_ERROR);
    let mut offset = 0_u64;
    let mut buffer = [0_u8; DIGEST_BUFFER_SIZE];
    while offset < length {
        let remaining = usize::try_from((length - offset).min(buffer.len() as u64))?;
        let count = source.read_at(&mut buffer[..remaining], offset)?;
        ensure!(count > 0, ARTIFACT_CHANGED_ERROR);
        destination.write_all(&buffer[..count])?;
        offset += count as u64;
    }
    Ok(())
}
