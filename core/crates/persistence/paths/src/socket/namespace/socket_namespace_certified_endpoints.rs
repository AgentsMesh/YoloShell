use super::super::{cleanup, socket_stage, unbound};
use crate::paths::{
    constants::{HOST_SOCKET_FILE_NAME, SOCKET_STAGE_LIFECYCLE_MAX_BYTES},
    directory::{PrivateDirectory, entry_c_string},
    private_file_lock::PrivateFileLock,
    socket_activation,
};
use anyhow::{Result, ensure};
use std::ffi::OsStr;
#[path = "certified_endpoints/inventory.rs"]
mod inventory;

const CHANGED: &str = "certified stale socket inventory changed";

/// Caller retains root lifecycle and exclusive durable ownership.
/// Every endpoint must first prove inactive; an unknown or live entry blocks all endpoint removal.
pub(super) fn cleanup(
    directory: &PrivateDirectory,
    fence: impl Fn() -> Result<()>,
) -> Result<bool> {
    fence()?;
    let target = socket_stage::Target::new(
        directory.path(),
        &entry_c_string(OsStr::new(HOST_SOCKET_FILE_NAME))?,
    )?;
    let Some(mut stage) = PrivateFileLock::try_acquire_in(
        directory.try_clone()?,
        &target.lifecycle_path(),
        SOCKET_STAGE_LIFECYCLE_MAX_BYTES,
    )?
    else {
        return Ok(false);
    };
    unbound::verify_stage(&stage)?;
    let expected = inventory::inspect(directory)?;
    #[cfg(all(test, feature = "yolo-integration-tests"))]
    super::test_hooks::run(super::test_hooks::Point::AfterEndpointInventory);
    for (name, identity) in &expected {
        fence()?;
        unbound::verify_stage(&stage)?;
        ensure!(inventory::identity(directory, name)? == *identity, CHANGED);
        if !socket_activation::prove_unbound_refused(directory, name, &stage)? {
            return Ok(false);
        }
        ensure!(inventory::identity(directory, name)? == *identity, CHANGED);
    }
    fence()?;
    unbound::verify_stage(&stage)?;
    ensure!(inventory::inspect(directory)? == expected, CHANGED);
    for (index, (name, identity)) in expected.iter().enumerate() {
        fence()?;
        unbound::verify_stage(&stage)?;
        ensure!(
            inventory::inspect(directory)?.as_slice() == &expected[index..],
            CHANGED
        );
        ensure!(
            socket_activation::prove_unbound_refused(directory, name, &stage)?,
            CHANGED
        );
        fence()?;
        unbound::verify_stage(&stage)?;
        ensure!(inventory::identity(directory, name)? == *identity, CHANGED);
        cleanup::unlink(directory.descriptor(), name, libc::c_int::default())?;
        directory.sync()?;
    }
    fence()?;
    unbound::verify_stage(&stage)?;
    ensure!(inventory::inspect(directory)?.is_empty(), CHANGED);
    stage.unlink_owned()?;
    directory.sync()?;
    Ok(true)
}
