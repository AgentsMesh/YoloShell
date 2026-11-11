use super::*;
use anyhow::{Context, Result};
use std::ffi::CStr;

const REMOVE_CONTEXT: &str = "remove socket namespace directory";
const RENAME_CONTEXT: &str = "quarantine uncertified socket namespace directory";

pub(super) fn rename_exclusive(
    root: &PrivateDirectory,
    source: &CStr,
    target: &CStr,
    lifecycle: &mut lifecycle_lock::SocketNamespaceLifecycleLock,
) -> Result<bool> {
    lifecycle.verify_root(root)?;
    match crate::paths::private_file::rename_exclusive_between(
        root.descriptor(),
        source,
        root.descriptor(),
        target,
    ) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
        Err(error) => return Err(error).context(RENAME_CONTEXT),
    }
    root.sync()?;
    lifecycle.verify_root(root)?;
    Ok(true)
}

pub(super) fn remove_directory(
    root: &PrivateDirectory,
    name: &CStr,
    lifecycle: &mut lifecycle_lock::SocketNamespaceLifecycleLock,
) -> Result<()> {
    lifecycle.verify_root(root)?;
    cleanup::unlink(root.descriptor(), name, libc::AT_REMOVEDIR).context(REMOVE_CONTEXT)?;
    root.sync()?;
    lifecycle.verify_root(root)
}
