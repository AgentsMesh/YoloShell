use super::cleanup_lease::{LeaseState, try_cleanup};
use super::lease::acquire_existing_lifecycle;
use super::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::directory_stream::DirectoryStream;
use anyhow::Result;
use std::path::Path;
#[path = "socket_stage_unlink_recovery.rs"]
mod unlink_recovery;
pub(crate) use unlink_recovery::{
    is_candidate as is_unlink_isolation, recover as recover_unlink_isolation,
};

#[derive(Debug)]
pub(crate) enum Cleanup {
    Busy,
    Mutated,
    Unchanged,
    Unproven,
}

pub(crate) fn cleanup_lifecycle(directory: &PrivateDirectory) -> Result<Cleanup> {
    let path = directory.path().join(LIFECYCLE_FILE_NAME);
    let lifecycle = match acquire_existing_lifecycle(directory, &path) {
        Ok(Some(lifecycle)) => lifecycle,
        Ok(None) => return Ok(Cleanup::Unchanged),
        Err(error) if record::is_unproven(&error) => return Ok(Cleanup::Unproven),
        Err(error) => return Err(error),
    };
    if has_bookkeeping(directory)? {
        return Ok(Cleanup::Unproven);
    }
    let mut lifecycle = lifecycle;
    lifecycle.unlink_owned()?;
    directory.sync()?;
    Ok(Cleanup::Mutated)
}

#[cfg(test)]
pub(crate) fn cleanup_key(directory: &PrivateDirectory, key: &str) -> Result<Cleanup> {
    cleanup_key_for_parent(directory, key, directory.path())
}

pub(crate) fn cleanup_key_for_parent(
    directory: &PrivateDirectory,
    key: &str,
    target_parent: &Path,
) -> Result<Cleanup> {
    cleanup_key_with_hooks(
        directory,
        key,
        target_parent,
        || Ok(()),
        || Ok(()),
        || Ok(()),
        || Ok(()),
    )
}

fn cleanup_key_with_hooks<F, G, H, I>(
    directory: &PrivateDirectory,
    key: &str,
    target_parent: &Path,
    mut before_quarantine: F,
    mut after_quarantine: G,
    mut before_probe_unlink: H,
    mut after_probe_unlink: I,
) -> Result<Cleanup>
where
    F: FnMut() -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut() -> Result<()>,
    I: FnMut() -> Result<()>,
{
    let lease = match try_cleanup(directory, key, target_parent)? {
        LeaseState::Acquired(lease) => *lease,
        LeaseState::Busy => return Ok(Cleanup::Busy),
        LeaseState::Unproven => return Ok(Cleanup::Unproven),
    };
    let mut hooks = probe_cleanup::Hooks {
        before_quarantine: &mut before_quarantine,
        after_quarantine: &mut after_quarantine,
        before_unlink: &mut before_probe_unlink,
        after_unlink: &mut after_probe_unlink,
    };
    if !residue_cleanup::cleanup_with_hooks(directory, lease.target(), None, &mut hooks)? {
        return Ok(Cleanup::Unproven);
    }
    if !lease.remove()? {
        return Ok(Cleanup::Busy);
    }
    Ok(Cleanup::Mutated)
}

#[cfg(test)]
pub(crate) fn cleanup_key_with_test_hooks<F, G>(
    directory: &PrivateDirectory,
    key: &str,
    target_parent: &Path,
    before_quarantine: F,
    before_probe_unlink: G,
) -> Result<Cleanup>
where
    F: FnMut() -> Result<()>,
    G: FnMut() -> Result<()>,
{
    cleanup_key_with_hooks(
        directory,
        key,
        target_parent,
        before_quarantine,
        || Ok(()),
        before_probe_unlink,
        || Ok(()),
    )
}

#[cfg(test)]
pub(crate) fn cleanup_key_with_all_test_hooks<F, G, H, I>(
    directory: &PrivateDirectory,
    key: &str,
    target_parent: &Path,
    before_quarantine: F,
    after_quarantine: G,
    before_probe_unlink: H,
    after_probe_unlink: I,
) -> Result<Cleanup>
where
    F: FnMut() -> Result<()>,
    G: FnMut() -> Result<()>,
    H: FnMut() -> Result<()>,
    I: FnMut() -> Result<()>,
{
    cleanup_key_with_hooks(
        directory,
        key,
        target_parent,
        before_quarantine,
        after_quarantine,
        before_probe_unlink,
        after_probe_unlink,
    )
}

fn has_bookkeeping(directory: &PrivateDirectory) -> Result<bool> {
    let mut stream = DirectoryStream::open(directory.descriptor())?;
    while let Some(name) = stream.next_name()? {
        let name = crate::paths::directory::entry_c_string(&name)?;
        if is_managed_stage(&name) || lease_key(&name).is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}
