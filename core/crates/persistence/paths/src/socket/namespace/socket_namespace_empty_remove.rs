use super::*;
use anyhow::{Context, Result};
use std::ffi::CStr;
use std::os::unix::ffi::OsStrExt;

const EMPTY_REMOVE_CONTEXT: &str = "remove empty socket namespace";

#[cfg(test)]
fn owned_with<F>(
    root: &PrivateDirectory,
    expected: &PrivateDirectory,
    name: &CStr,
    before_final_fence: F,
) -> Result<bool>
where
    F: FnOnce() -> Result<()>,
{
    let Some(mut lifecycle) = lifecycle_lock::SocketNamespaceLifecycleLock::try_acquire(root)?
    else {
        return Ok(false);
    };
    owned_locked_with(root, expected, name, &mut lifecycle, before_final_fence)
}

pub(super) fn owned_locked(
    root: &PrivateDirectory,
    expected: &PrivateDirectory,
    name: &CStr,
    lifecycle: &mut lifecycle_lock::SocketNamespaceLifecycleLock,
) -> Result<bool> {
    owned_locked_with(root, expected, name, lifecycle, || Ok(()))
}

fn owned_locked_with<F>(
    root: &PrivateDirectory,
    expected: &PrivateDirectory,
    name: &CStr,
    lifecycle: &mut lifecycle_lock::SocketNamespaceLifecycleLock,
    before_final_fence: F,
) -> Result<bool>
where
    F: FnOnce() -> Result<()>,
{
    let path = root
        .path()
        .join(std::ffi::OsStr::from_bytes(name.to_bytes()));
    if !matches_expected(root, expected, &path, name)? {
        return Ok(false);
    }
    before_final_fence()?;
    if !matches_expected(root, expected, &path, name)? {
        return Ok(false);
    }
    root_mutation::remove_directory(root, name, lifecycle).context(EMPTY_REMOVE_CONTEXT)?;
    Ok(true)
}

fn matches_expected(
    root: &PrivateDirectory,
    expected: &PrivateDirectory,
    path: &Path,
    name: &CStr,
) -> Result<bool> {
    let current = match candidate::open(root, path, name)? {
        candidate::Candidate::Missing | candidate::Candidate::Unsafe => return Ok(false),
        candidate::Candidate::Open(current) => current,
    };
    Ok(current.same_identity(expected) && cleanup::is_empty(current.descriptor())?)
}

#[cfg(test)]
pub(super) fn owned_with_test_hook<F>(
    root: &PrivateDirectory,
    expected: &PrivateDirectory,
    name: &CStr,
    hook: F,
) -> Result<bool>
where
    F: FnOnce() -> Result<()>,
{
    owned_with(root, expected, name, hook)
}
