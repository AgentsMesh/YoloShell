use anyhow::{Result, ensure};
use std::ffi::CString;
use std::os::unix::fs::MetadataExt;
use yolo_release_contracts::{PERMISSION_BITS_MASK, UNSAFE_DIRECTORY_WRITE_MASK};
use yolo_release_filesystem::DirectoryHandle;
use yolo_release_filesystem::ensure_no_extended_acl;

pub fn validate_parent(directory: &DirectoryHandle) -> Result<()> {
    directory.verify_path()?;
    let metadata = directory.retained_file().metadata()?;
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "candidate output parent must be owned by the current user"
    );
    ensure!(
        directory.mode()? & UNSAFE_DIRECTORY_WRITE_MASK == 0,
        "candidate output parent must not be group or world writable"
    );
    ensure_no_extended_acl(directory.raw_fd(), "candidate output parent")
}

pub fn validate_directory(
    directory: &DirectoryHandle,
    description: &str,
    mode: u32,
    flags: u32,
) -> Result<()> {
    let metadata = directory.retained_file().metadata()?;
    ensure!(
        metadata.uid() == unsafe { libc::geteuid() },
        "{description} must be owned by the current user"
    );
    ensure!(
        directory.mode()? & PERMISSION_BITS_MASK == mode,
        "{description} mode mismatch"
    );
    ensure!(directory.flags()? == flags, "{description} flags mismatch");
    ensure_no_extended_acl(directory.raw_fd(), description)
}

pub fn ensure_missing(parent: &DirectoryHandle, name: &str) -> Result<()> {
    ensure!(
        !entry_exists(parent, name)?,
        "candidate output must initially be absent"
    );
    Ok(())
}

pub fn entry_exists(parent: &DirectoryHandle, name: &str) -> Result<bool> {
    let name = CString::new(name)?;
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let status = unsafe {
        libc::fstatat(
            parent.raw_fd(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if status == 0 {
        return Ok(true);
    }
    let error = std::io::Error::last_os_error();
    ensure!(
        error.raw_os_error() == Some(libc::ENOENT),
        "inspect candidate output: {error}"
    );
    Ok(false)
}
