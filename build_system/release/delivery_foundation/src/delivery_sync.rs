use anyhow::{Context, Result, ensure};
use std::ffi::CString;
use std::fs::File;
use std::os::fd::FromRawFd;
use yolo_release_contracts::{DELIVERY_OPEN_FAILURE, DELIVERY_SYNC_ERROR};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_gate_system::entry_names;

pub fn sync_tree(directory: &DirectoryHandle) -> Result<()> {
    directory.verify_self()?;
    for name in entry_names(directory)? {
        let file = open_entry(directory, &name)?;
        let metadata = file.metadata()?;
        if metadata.is_dir() {
            let child = DirectoryHandle::open_at_bytes(directory, &name)?;
            sync_tree(&child)?;
        } else {
            ensure!(metadata.is_file(), DELIVERY_SYNC_ERROR);
            file.sync_all().context(DELIVERY_SYNC_ERROR)?;
        }
    }
    directory.sync_all().context(DELIVERY_SYNC_ERROR)
}

fn open_entry(parent: &DirectoryHandle, name: &[u8]) -> Result<File> {
    let name = CString::new(name)?;
    let descriptor = unsafe {
        libc::openat(
            parent.raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if descriptor == DELIVERY_OPEN_FAILURE {
        return Err(std::io::Error::last_os_error()).context(DELIVERY_SYNC_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}
