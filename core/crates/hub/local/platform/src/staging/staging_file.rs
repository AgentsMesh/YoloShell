use super::internal;
use std::ffi::CString;
use std::fs::File;
use std::os::{
    fd::FromRawFd,
    unix::fs::{FileExt, MetadataExt},
};
use yolo_hub_terminal_protocol::{PublicError, StageAttachmentRequest};
use yolo_paths::directory::PrivateDirectory;

const NON_OWNER_PERMISSIONS: u32 = 0o077;
const MODE: libc::mode_t = 0o600;

pub(super) fn apply(
    directory: &PrivateDirectory,
    request: &StageAttachmentRequest,
    complete: bool,
) -> Result<(), PublicError> {
    let name = CString::new(request.name.as_bytes()).map_err(|_| PublicError::InvalidMetadata)?;
    let access = if complete {
        libc::O_RDONLY
    } else {
        libc::O_RDWR | libc::O_CREAT
    };
    let flags = access | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK;
    // openat resolves this validated single component against the retained private directory.
    let fd = unsafe {
        libc::openat(
            directory.descriptor(),
            name.as_ptr(),
            flags,
            MODE as libc::c_int,
        )
    };
    if fd < 0 {
        let error = std::io::Error::last_os_error();
        return Err(if error.raw_os_error() == Some(libc::ENAMETOOLONG) {
            PublicError::InvalidMetadata
        } else {
            internal(error)
        });
    }
    let file = unsafe { File::from_raw_fd(fd) };
    let stat = file.metadata().map_err(internal)?;
    if !stat.is_file()
        || stat.nlink() != 1
        || stat.uid() != unsafe { libc::geteuid() }
        || stat.mode() & NON_OWNER_PERMISSIONS != 0
    {
        return Err(PublicError::Conflict);
    }
    let end = request.next_offset()?;
    if request.offset > stat.len() {
        return Err(PublicError::InvalidCursor);
    }
    if (request.eof && end < stat.len()) || (complete && end > stat.len()) {
        return Err(PublicError::Conflict);
    }
    // Replay or repair a short write by verifying every overlapping byte first.
    let overlap = (stat.len().min(end) - request.offset) as usize;
    if overlap > 0 {
        let mut existing = vec![0; overlap];
        file.read_exact_at(&mut existing, request.offset)
            .map_err(internal)?;
        if existing != request.data[..overlap] {
            return Err(PublicError::Conflict);
        }
    }
    if overlap < request.data.len() {
        file.write_all_at(&request.data[overlap..], stat.len())
            .map_err(internal)?;
    }
    if request.eof && !complete {
        file.sync_all().map_err(internal)?;
        directory.sync().map_err(internal)?;
    }
    Ok(())
}
