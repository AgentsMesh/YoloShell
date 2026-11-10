use super::*;
use anyhow::{Context, Result};
use std::ffi::CStr;
use std::mem::MaybeUninit;

const SOCKET_IDENTITY_CONTEXT: &str = "inspect socket namespace endpoint";

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) struct SocketIdentity {
    device: libc::dev_t,
    inode: libc::ino_t,
}

pub(super) fn inspect_socket(
    directory: &PrivateDirectory,
    name: &CStr,
) -> Result<Option<SocketIdentity>> {
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
            Ok(None)
        } else {
            Err(error).context(SOCKET_IDENTITY_CONTEXT)
        };
    }
    let metadata = unsafe { metadata.assume_init() };
    let mode = u32::from(metadata.st_mode);
    if metadata.st_mode & libc::S_IFMT != libc::S_IFSOCK
        || metadata.st_uid != unsafe { libc::geteuid() }
        || u64::from(metadata.st_nlink) != SOCKET_ENDPOINT_LINK_COUNT
        || mode & PERMISSION_MODE_MASK != PRIVATE_FILE_MODE
    {
        return Ok(None);
    }
    Ok(Some(SocketIdentity {
        device: metadata.st_dev,
        inode: metadata.st_ino,
    }))
}
