use super::RemoteError;
use super::constants::*;
use std::fs::{Metadata, OpenOptions};
use std::io::{self, Read};
use std::path::Path;

#[path = "local_file_identity.rs"]
mod identity;

const EXPECTED_LOCAL_FILE_LINKS: u64 = 1;
const LOCAL_FILE_SIZE_SENTINEL_BYTES: u64 = 1;
const ROOT_USER_ID: u32 = 0;

pub(super) fn read_known_hosts(path: &Path) -> Result<String, RemoteError> {
    read_known_hosts_with(path, |_| Ok(()))
}

fn read_known_hosts_with<F>(path: &Path, after_read: F) -> Result<String, RemoteError>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let bytes = read_bounded(
        path,
        MAX_KNOWN_HOSTS_BYTES,
        KNOWN_HOSTS_DENIED_MODE,
        true,
        after_read,
    )
    .map_err(RemoteError::KnownHostsIo)?;
    String::from_utf8(bytes).map_err(|error| {
        RemoteError::KnownHostsIo(io::Error::new(io::ErrorKind::InvalidData, error))
    })
}

pub(super) fn read_identity(path: &Path) -> Result<String, RemoteError> {
    read_identity_with(path, |_| Ok(()))
}

fn read_identity_with<F>(path: &Path, after_read: F) -> Result<String, RemoteError>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let bytes = read_bounded(
        path,
        MAX_IDENTITY_FILE_BYTES,
        IDENTITY_DENIED_MODE,
        false,
        after_read,
    )
    .map_err(|error| RemoteError::transport(format!("SSH identity read failed: {error}")))?;
    String::from_utf8(bytes)
        .map_err(|error| RemoteError::transport(format!("SSH identity is not UTF-8: {error}")))
}

fn read_bounded(
    path: &Path,
    limit: usize,
    denied_mode: u32,
    allow_root: bool,
    after_read: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<Vec<u8>> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    validate_acl(&file, path)?;
    let before = file.metadata()?;
    validate_metadata(&before, denied_mode, allow_root)?;
    ensure_limit(before.len(), limit)?;
    let mut bytes = Vec::with_capacity(before.len() as usize);
    (&file)
        .take(limit as u64 + LOCAL_FILE_SIZE_SENTINEL_BYTES)
        .read_to_end(&mut bytes)?;
    ensure_limit(bytes.len() as u64, limit)?;
    after_read(path)?;
    validate_acl(&file, path)?;
    let after = file.metadata()?;
    validate_metadata(&after, denied_mode, allow_root)?;
    identity::ensure_stable(path, &before, &after, bytes.len())?;
    Ok(bytes)
}

fn validate_acl(file: &std::fs::File, path: &Path) -> io::Result<()> {
    crate::private_acl::require_no_grants(file, path)
        .map_err(|error| io::Error::new(io::ErrorKind::PermissionDenied, error))
}

fn ensure_limit(length: u64, limit: usize) -> io::Result<()> {
    if length <= limit as u64 {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "local file exceeds its size limit",
        ))
    }
}

fn validate_metadata(metadata: &Metadata, denied_mode: u32, allow_root: bool) -> io::Result<()> {
    if !metadata.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "local input is not a regular file",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let owner = metadata.uid();
        let expected = unsafe { libc::geteuid() };
        let mode = metadata.permissions().mode() & LOCAL_FILE_PERMISSION_MASK;
        if !owner_and_mode_are_safe(owner, expected, mode, denied_mode, allow_root) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "local input owner or mode is unsafe",
            ));
        }
        if metadata.nlink() != EXPECTED_LOCAL_FILE_LINKS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "local input has multiple links",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn owner_and_mode_are_safe(
    owner: u32,
    expected: u32,
    mode: u32,
    denied_mode: u32,
    allow_root: bool,
) -> bool {
    (owner == expected || (allow_root && owner == ROOT_USER_ID)) && mode & denied_mode == 0
}

#[cfg(test)]
pub(super) fn read_identity_for_test(path: &Path) -> Result<String, RemoteError> {
    read_identity(path)
}

#[cfg(test)]
pub(super) fn read_identity_with_test_hook<F>(path: &Path, hook: F) -> Result<String, RemoteError>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    read_identity_with(path, hook)
}

#[cfg(test)]
pub(super) fn read_known_hosts_with_test_hook<F>(
    path: &Path,
    hook: F,
) -> Result<String, RemoteError>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    read_known_hosts_with(path, hook)
}

#[cfg(all(test, unix))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/local_file_policy_tests.rs"]
mod policy_tests;
