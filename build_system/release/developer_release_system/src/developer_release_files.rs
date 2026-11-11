use crate::delivery_files::ProtectedFile;
use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::FromRawFd;
use yolo_release_contracts::{DELIVERY_OPEN_FAILURE, DELIVERY_SYSTEM_CALL_SUCCESS};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::digest::{MAX_DIGEST_INPUT_BYTES, copy_opened_bounded};

const REMOVE_CONTEXT: &str = "remove private Developer release file";
const COPY_CONTEXT: &str = "copy executable into private signature normalization";

pub fn remove(root: &DirectoryHandle, name: &str, protected: &ProtectedFile) -> Result<()> {
    let reopened = protected.verify_at(root, name)?;
    let name = CString::new(name)?;
    let status = unsafe { libc::unlinkat(root.raw_fd(), name.as_ptr(), 0) };
    if status == DELIVERY_OPEN_FAILURE {
        return Err(std::io::Error::last_os_error()).context(REMOVE_CONTEXT);
    }
    ensure!(status == DELIVERY_SYSTEM_CALL_SUCCESS, REMOVE_CONTEXT);
    protected.file().metadata()?;
    reopened.metadata()?;
    root.verify_self()
}

pub fn remove_named_archive(root: &DirectoryHandle) -> Result<()> {
    let archive = ProtectedFile::open(root, yolo_release_contracts::ARCHIVE_NAME)?;
    remove(root, yolo_release_contracts::ARCHIVE_NAME, &archive)
}

pub fn copy_writable(root: &DirectoryHandle, name: &str, source: &File) -> Result<()> {
    root.verify_self()?;
    let name = CString::new(name)?;
    let descriptor = unsafe {
        libc::openat(
            root.raw_fd(),
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            yolo_release_contracts::PRIVATE_FILE_MODE,
        )
    };
    if descriptor == DELIVERY_OPEN_FAILURE {
        return Err(std::io::Error::last_os_error()).context(COPY_CONTEXT);
    }
    let mut destination = unsafe { File::from_raw_fd(descriptor) };
    copy_opened_bounded(
        source,
        &mut destination,
        MAX_DIGEST_INPUT_BYTES,
        COPY_CONTEXT,
    )?;
    destination.sync_all()?;
    root.verify_self()
}
