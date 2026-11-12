use super::retained_artifact::RetainedArtifact;
use super::secure_vfs_identity::FileIdentity;
use anyhow::{Context, Result};
use rusqlite::ffi;
use std::fs::File;
use std::os::fd::FromRawFd;
use std::os::unix::fs::PermissionsExt;

const SQLITE_ARTIFACT_MODE: u32 = 0o600;
const PERMISSION_MODE_MASK: u32 = 0o7777;

pub(super) fn access(artifact: &RetainedArtifact, request: libc::c_int) -> Result<bool> {
    let mode = match request {
        ffi::SQLITE_ACCESS_EXISTS | ffi::SQLITE_ACCESS_READ => libc::O_RDONLY,
        ffi::SQLITE_ACCESS_READWRITE => libc::O_RDWR,
        _ => anyhow::bail!("invalid retained SQLite access request"),
    };
    let flags = mode | libc::O_CLOEXEC | libc::O_NOFOLLOW;
    let entry = artifact.entry();
    let descriptor =
        unsafe { libc::openat(entry.parent_descriptor(), entry.file_name().as_ptr(), flags) };
    if descriptor < 0 {
        let error = std::io::Error::last_os_error();
        return if error.kind() == std::io::ErrorKind::NotFound {
            Ok(false)
        } else {
            Err(error).context("access retained SQLite artifact")
        };
    }
    let file = unsafe { File::from_raw_fd(descriptor) };
    let metadata = file.metadata()?;
    let identity = FileIdentity::inspect(&metadata, artifact.entry().path())?;
    artifact.verify(identity)?;
    crate::private_acl::require_empty(&file, artifact.entry().path())?;
    anyhow::ensure!(
        metadata.permissions().mode() & PERMISSION_MODE_MASK == SQLITE_ARTIFACT_MODE,
        "retained SQLite artifact mode is unsafe"
    );
    Ok(true)
}
