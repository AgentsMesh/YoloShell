use std::ffi::CStr;
use std::fs::File;
use std::os::fd::{FromRawFd, RawFd};
use yolo_hub_terminal_protocol::PublicError;
use yolo_paths::identity::ObjectIdentity;

pub(crate) fn open_at(parent: RawFd, name: &CStr, flags: libc::c_int) -> Result<File, PublicError> {
    let descriptor = unsafe { libc::openat(parent, name.as_ptr(), flags) };
    if descriptor < 0 {
        Err(io_error(std::io::Error::last_os_error()))
    } else {
        Ok(unsafe { File::from_raw_fd(descriptor) })
    }
}

pub(crate) fn identity(file: &File) -> Result<ObjectIdentity, PublicError> {
    ObjectIdentity::from_file(file)
        .map_err(|_| PublicError::Unreachable)?
        .ok_or(PublicError::Unsupported)
}

pub(crate) fn io_error(error: std::io::Error) -> PublicError {
    match error.raw_os_error() {
        Some(libc::ELOOP | libc::ENOTDIR) => return PublicError::Unauthorized,
        _ => {}
    }
    match error.kind() {
        std::io::ErrorKind::NotFound => PublicError::NotFound,
        std::io::ErrorKind::PermissionDenied => PublicError::Unauthorized,
        std::io::ErrorKind::UnexpectedEof => PublicError::Conflict,
        _ => PublicError::Unreachable,
    }
}
