use crate::paths::private_file::PrivateFileEntry;
use std::fs::File;
use std::os::fd::FromRawFd;

const SQLITE_ARTIFACT_MODE: libc::c_uint = 0o600;

pub(super) fn open(entry: &PrivateFileEntry, flags: libc::c_int) -> std::io::Result<File> {
    let descriptor = unsafe {
        libc::openat(
            entry.parent_descriptor(),
            entry.file_name().as_ptr(),
            flags,
            SQLITE_ARTIFACT_MODE,
        )
    };
    if descriptor < 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

pub(super) fn reopen(entry: &PrivateFileEntry) -> std::io::Result<File> {
    open(entry, libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW)
}

pub(super) fn link_at(
    entry: &PrivateFileEntry,
    target_parent: libc::c_int,
    target_name: &std::ffi::CStr,
) -> std::io::Result<()> {
    let result = unsafe {
        libc::linkat(
            entry.parent_descriptor(),
            entry.file_name().as_ptr(),
            target_parent,
            target_name.as_ptr(),
            0,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
