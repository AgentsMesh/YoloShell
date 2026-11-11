use crate::paths::constants::{PERMISSION_MODE_MASK, PRIVATE_FILE_MODE};
use crate::paths::directory::PrivateDirectory;
use anyhow::{Context, Result};
use std::ffi::CStr;
use std::mem::MaybeUninit;

const INSPECT_CONTEXT: &str = "inspect managed socket publication stage";
const REMOVE_CONTEXT: &str = "remove managed socket publication stage";
const RENAME_CONTEXT: &str = "quarantine managed socket probe alias";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Identity {
    pub(super) device: libc::dev_t,
    pub(super) inode: libc::ino_t,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SocketState {
    pub(super) identity: Identity,
    pub(super) links: u64,
}

pub(super) enum Inspection {
    Missing,
    Socket(SocketState),
    Unsafe,
}

pub(super) fn inspect(directory: &PrivateDirectory, name: &CStr) -> Result<Inspection> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory.descriptor(),
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result != 0 {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::NotFound {
            Ok(Inspection::Missing)
        } else {
            Err(error).context(INSPECT_CONTEXT)
        };
    }
    let metadata = unsafe { metadata.assume_init() };
    let mode = u32::from(metadata.st_mode);
    if metadata.st_mode & libc::S_IFMT != libc::S_IFSOCK
        || metadata.st_uid != unsafe { libc::geteuid() }
        || mode & PERMISSION_MODE_MASK != PRIVATE_FILE_MODE
    {
        return Ok(Inspection::Unsafe);
    }
    Ok(Inspection::Socket(SocketState {
        identity: Identity {
            device: metadata.st_dev,
            inode: metadata.st_ino,
        },
        links: u64::from(metadata.st_nlink),
    }))
}

pub(super) fn unlink(directory: &PrivateDirectory, name: &CStr) -> Result<()> {
    let result = unsafe { libc::unlinkat(directory.descriptor(), name.as_ptr(), 0) };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error()).context(REMOVE_CONTEXT)
    }
}

pub(super) fn rename_exclusive(
    directory: &PrivateDirectory,
    source: &CStr,
    target: &CStr,
) -> Result<bool> {
    let result = crate::paths::private_file::rename_exclusive_between(
        directory.descriptor(),
        source,
        directory.descriptor(),
        target,
    );
    match result {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error).context(RENAME_CONTEXT),
    }
}
