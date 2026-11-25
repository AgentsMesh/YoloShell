use super::constants::*;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::os::fd::RawFd;
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SocketIdentity {
    device: u64,
    inode: u64,
}

pub(super) fn socket_identity(path: &Path) -> std::io::Result<SocketIdentity> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = socket_metadata(path)?;
    if metadata.permissions().mode() & SOCKET_PERMISSION_MODE_MASK != PRIVATE_FILE_MODE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            BROKER_SOCKET_PERMISSION_ERROR,
        ));
    }
    Ok(identity_from_metadata(&metadata))
}

pub(super) fn socket_identity_at(
    directory: RawFd,
    name: &CStr,
) -> std::io::Result<Option<SocketIdentity>> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return identity_from_stat(&unsafe { metadata.assume_init() }, true).map(Some);
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::NotFound {
        Ok(None)
    } else {
        Err(error)
    }
}

pub(super) fn bound_socket_identity_at(
    directory: RawFd,
    name: &CStr,
) -> std::io::Result<Option<SocketIdentity>> {
    let mut metadata = MaybeUninit::<libc::stat>::uninit();
    let result = unsafe {
        libc::fstatat(
            directory,
            name.as_ptr(),
            metadata.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        identity_from_stat(&unsafe { metadata.assume_init() }, false).map(Some)
    } else {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            Ok(None)
        } else {
            Err(error)
        }
    }
}

fn identity_from_stat(
    metadata: &libc::stat,
    require_private_mode: bool,
) -> std::io::Result<SocketIdentity> {
    let mode = metadata.st_mode as u32;
    if mode & libc::S_IFMT as u32 != libc::S_IFSOCK as u32 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            BROKER_SOCKET_TYPE_ERROR,
        ));
    }
    if metadata.st_uid != unsafe { libc::geteuid() } {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            BROKER_SOCKET_OWNER_ERROR,
        ));
    }
    if require_private_mode && mode & SOCKET_PERMISSION_MODE_MASK != PRIVATE_FILE_MODE {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            BROKER_SOCKET_PERMISSION_ERROR,
        ));
    }
    Ok(SocketIdentity {
        device: metadata.st_dev.try_into().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                BROKER_SOCKET_DEVICE_INVALID,
            )
        })?,
        inode: metadata.st_ino,
    })
}

fn socket_metadata(path: &Path) -> std::io::Result<std::fs::Metadata> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.file_type().is_socket() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            BROKER_SOCKET_TYPE_ERROR,
        ));
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            BROKER_SOCKET_OWNER_ERROR,
        ));
    }
    Ok(metadata)
}

fn identity_from_metadata(metadata: &std::fs::Metadata) -> SocketIdentity {
    use std::os::unix::fs::MetadataExt;
    SocketIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/socket_identity_tests.rs"]
mod tests;
