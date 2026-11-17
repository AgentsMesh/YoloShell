use super::model::valid_id;
use anyhow::{Result, ensure};
use std::{
    ffi::CString,
    fs::File,
    io::Read,
    os::{fd::FromRawFd, unix::fs::MetadataExt},
    path::Path,
};
use yolo_release_contracts::*;
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_foundation::file_seal::FileSeal;

pub(super) fn read(parent: &DirectoryHandle, path: &Path) -> Result<Option<String>> {
    parent.verify_path()?;
    ensure!(
        path.parent() == Some(parent.display_path()),
        CONTAINER_CID_INVALID
    );
    let name = CString::new(
        path.file_name()
            .ok_or_else(|| anyhow::anyhow!(CONTAINER_CID_INVALID))?
            .as_encoded_bytes(),
    )?;
    let fd = unsafe {
        libc::openat(
            parent.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error.into())
        };
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file()
            && metadata.nlink() == EXPECTED_FILE_LINKS
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & UNSAFE_DIRECTORY_WRITE_MASK == 0
            && metadata.len() <= (CONTAINER_ID_BYTES + CONTAINER_CID_NEWLINE_BYTES) as u64,
        CONTAINER_CID_INVALID
    );
    yolo_release_filesystem::ensure_no_extended_acl(fd, CONTAINER_CID_INVALID)?;
    let seal = FileSeal::capture(&file)?;
    let mut bytes = Vec::new();
    (&file)
        .take(
            (CONTAINER_ID_BYTES + CONTAINER_CID_NEWLINE_BYTES + CONTAINER_CID_READ_SENTINEL_BYTES)
                as u64,
        )
        .read_to_end(&mut bytes)?;
    seal.verify(&file, CONTAINER_CID_INVALID)?;
    parent.verify_path()?;
    let reopened = unsafe {
        libc::openat(
            parent.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
        )
    };
    ensure!(reopened >= 0, CONTAINER_CID_INVALID);
    let reopened = unsafe { File::from_raw_fd(reopened) };
    seal.verify(&reopened, CONTAINER_CID_INVALID)?;
    let value = std::str::from_utf8(&bytes)?.trim_end_matches('\n');
    // Docker reserves the cidfile before sending create, so an empty file is
    // not positive acknowledgement and must use exact-name recovery instead.
    if value.is_empty() {
        return Ok(None);
    }
    ensure!(valid_id(value), CONTAINER_CID_INVALID);
    Ok(Some(value.into()))
}
