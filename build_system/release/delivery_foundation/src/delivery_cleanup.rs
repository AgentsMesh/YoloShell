use anyhow::{Context, Result, ensure};
use std::ffi::{CStr, CString};
use std::fs::File;
use std::os::fd::FromRawFd;
use yolo_release_contracts::{
    DELIVERY_CLEANUP_ERROR, DELIVERY_OPEN_FAILURE, DELIVERY_SYSTEM_CALL_SUCCESS,
};
use yolo_release_contracts::{NO_FILE_FLAGS, PRIVATE_DIRECTORY_MODE};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_gate_system::entry_names;

pub fn remove_stage(parent: &DirectoryHandle, name: &CStr, stage: &DirectoryHandle) -> Result<()> {
    parent.verify_child_bytes(name.to_bytes(), stage)?;
    clear(stage)?;
    ensure!(
        unsafe { libc::unlinkat(parent.raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) }
            == DELIVERY_SYSTEM_CALL_SUCCESS,
        DELIVERY_CLEANUP_ERROR
    );
    parent.verify_self()
}

fn clear(directory: &DirectoryHandle) -> Result<()> {
    directory.set_flags(NO_FILE_FLAGS)?;
    directory.set_mode(PRIVATE_DIRECTORY_MODE)?;
    for name in entry_names(directory)? {
        let metadata = open_entry(directory, &name)?.metadata()?;
        let name = CString::new(name)?;
        if metadata.is_dir() {
            let child = DirectoryHandle::open_at_bytes(directory, name.as_bytes())?;
            clear(&child)?;
            directory.verify_child_bytes(name.as_bytes(), &child)?;
            ensure!(
                unsafe { libc::unlinkat(directory.raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR) }
                    == DELIVERY_SYSTEM_CALL_SUCCESS,
                DELIVERY_CLEANUP_ERROR
            );
        } else {
            ensure!(
                unsafe { libc::unlinkat(directory.raw_fd(), name.as_ptr(), 0) }
                    == DELIVERY_SYSTEM_CALL_SUCCESS,
                DELIVERY_CLEANUP_ERROR
            );
        }
    }
    directory.verify_self()
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
        return Err(std::io::Error::last_os_error()).context(DELIVERY_CLEANUP_ERROR);
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}
