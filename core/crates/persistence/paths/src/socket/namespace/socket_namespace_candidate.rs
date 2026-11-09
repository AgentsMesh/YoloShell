use super::*;
use anyhow::{Context, Result};
use std::ffi::CStr;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::fs::MetadataExt;

const CANDIDATE_CHANGED: &str = "socket namespace candidate changed during inspection";

pub(super) enum Candidate {
    Missing,
    Open(PrivateDirectory),
    Unsafe,
}

pub(super) fn open(root: &PrivateDirectory, path: &Path, name: &CStr) -> Result<Candidate> {
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    let descriptor = unsafe { libc::openat(root.descriptor(), name.as_ptr(), flags) };
    if descriptor < 0 {
        let error = std::io::Error::last_os_error();
        return match error.kind() {
            std::io::ErrorKind::NotFound => Ok(Candidate::Missing),
            _ if unsafe_open_error(&error) => Ok(Candidate::Unsafe),
            _ => Err(error).context(CANDIDATE_CHANGED),
        };
    }
    classify_opened(unsafe { File::from_raw_fd(descriptor) }, path)
}

fn classify_opened(file: File, path: &Path) -> Result<Candidate> {
    let metadata = file.metadata().context(CANDIDATE_CHANGED)?;
    if metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & PERMISSION_MODE_MASK != PRIVATE_DIRECTORY_MODE
    {
        return Ok(Candidate::Unsafe);
    }
    if let Err(error) = crate::private_acl::require_empty(&file, path) {
        return classify_validation_error(error);
    }
    match PrivateDirectory::from_opened(file, path) {
        Ok(directory) => Ok(Candidate::Open(directory)),
        Err(error) => classify_validation_error(error),
    }
}

pub(super) fn classify_validation_error(error: anyhow::Error) -> Result<Candidate> {
    if error.chain().any(|source| source.is::<std::io::Error>()) {
        Err(error).context(CANDIDATE_CHANGED)
    } else {
        Ok(Candidate::Unsafe)
    }
}

fn unsafe_open_error(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::NotADirectory
    ) || error.raw_os_error() == Some(libc::ELOOP)
}

pub(super) fn same_current(
    root: &PrivateDirectory,
    path: &Path,
    name: &CStr,
    expected: &PrivateDirectory,
) -> Result<bool> {
    match open(root, path, name)? {
        Candidate::Open(current) => Ok(current.same_identity(expected)),
        Candidate::Missing | Candidate::Unsafe => Ok(false),
    }
}
