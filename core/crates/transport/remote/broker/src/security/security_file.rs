use super::constants::{EXPECTED_SECURE_FILE_LINKS, PERMISSION_BITS_MASK};
use anyhow::{Result, ensure};
use std::fs::{File, Metadata};
use std::path::Path;

pub(super) fn inspect_secure_file(
    file: &File,
    path: &Path,
    label: &str,
    denied_mode: u32,
    allow_root: bool,
    limit: usize,
) -> Result<Metadata> {
    crate::private_acl::require_no_grants(file, path)?;
    let metadata = file.metadata()?;
    ensure!(
        metadata.is_file(),
        "{label} is not a regular file: {}",
        path.display()
    );
    inspect_unix_metadata(&metadata, path, label, denied_mode, allow_root)?;
    ensure!(
        metadata.len() <= limit as u64,
        "{label} exceeds its size limit"
    );
    Ok(metadata)
}

#[cfg(unix)]
fn inspect_unix_metadata(
    metadata: &Metadata,
    path: &Path,
    label: &str,
    denied_mode: u32,
    allow_root: bool,
) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let owner = metadata.uid();
    ensure!(
        owner == effective_uid() || (allow_root && owner == 0),
        "{label} has unsafe owner: {}",
        path.display()
    );
    let mode = metadata.permissions().mode() & PERMISSION_BITS_MASK;
    ensure!(
        mode & denied_mode == 0,
        "{label} has unsafe permissions: {mode:o}"
    );
    ensure!(
        metadata.nlink() == EXPECTED_SECURE_FILE_LINKS,
        "{label} has multiple links: {}",
        path.display()
    );
    Ok(())
}

pub(super) fn verify_secure_file_stable(
    path: &Path,
    label: &str,
    before: &Metadata,
    after: &Metadata,
    read: usize,
) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    let published = std::fs::symlink_metadata(path)?;
    ensure!(
        before.dev() == after.dev()
            && before.ino() == after.ino()
            && before.len() == after.len()
            && before.mtime() == after.mtime()
            && before.mtime_nsec() == after.mtime_nsec()
            && before.ctime() == after.ctime()
            && before.ctime_nsec() == after.ctime_nsec()
            && after.len() == read as u64
            && published.dev() == after.dev()
            && published.ino() == after.ino(),
        "{label} changed while being read: {}",
        path.display()
    );
    Ok(())
}

#[cfg(unix)]
pub(super) fn effective_uid() -> libc::uid_t {
    // SAFETY: geteuid only reads the process credentials and has no preconditions.
    unsafe { libc::geteuid() }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/security/security_file_branch_tests.rs"]
mod branch_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../../unit_tests/security/security_file_tests.rs"]
mod tests;
