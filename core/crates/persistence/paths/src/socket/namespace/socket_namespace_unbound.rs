use super::{SocketNamespaceLocation, lifecycle_lock::SocketNamespaceLifecycleLock, root_mutation};
use crate::paths::{
    constants::{HOST_SOCKET_FILE_NAME, SOCKET_STAGE_LIFECYCLE_MAX_BYTES},
    directory::{PrivateDirectory, entry_c_string},
    private_file_lock::{PrivateFileLock, RetainedPrivateFileVerification},
    socket_stage,
};
use anyhow::{Context, Result, ensure};
use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
#[path = "socket_namespace_unbound_constants.rs"]
mod constants;
#[path = "socket_namespace_unbound_inventory.rs"]
mod inventory;
use constants::*;

/// Caller holds the root lifecycle and an exclusive durable registration.
/// Preserves every original entry while recovering interrupted certification.
pub(super) fn recover(
    root: &PrivateDirectory,
    location: &SocketNamespaceLocation,
    directory: &PrivateDirectory,
    lifecycle: &mut SocketNamespaceLifecycleLock,
    fence: impl Fn() -> Result<()>,
) -> Result<()> {
    recover_with(root, location, directory, lifecycle, fence, || Ok(()))
}

fn recover_with(
    root: &PrivateDirectory,
    location: &SocketNamespaceLocation,
    directory: &PrivateDirectory,
    lifecycle: &mut SocketNamespaceLifecycleLock,
    fence: impl Fn() -> Result<()>,
    before_quarantine: impl FnOnce() -> Result<()>,
) -> Result<()> {
    ensure!(
        directory.path() == location.directory && location.directory.parent() == Some(root.path()),
        CHANGED
    );
    root.verify_path()?;
    directory.verify_path()?;
    fence()?;
    require_uncertified(directory)?;
    let expected = inventory::inspect(directory)?;
    let target = socket_stage::Target::new(
        directory.path(),
        &entry_c_string(OsStr::new(HOST_SOCKET_FILE_NAME))?,
    )?;
    let stage_path = target.lifecycle_path();
    let stage = PrivateFileLock::try_acquire_in(
        directory.try_clone()?,
        &stage_path,
        SOCKET_STAGE_LIFECYCLE_MAX_BYTES,
    )?
    .context(STAGE_BUSY)?;
    verify_stage(&stage)?;
    ensure!(inventory::inspect(directory)? == expected, CHANGED);
    for (name, _) in &expected {
        fence()?;
        verify_stage(&stage)?;
        ensure!(
            crate::paths::socket_activation::prove_unbound_refused(directory, name, &stage)?,
            UNPROVEN
        );
    }
    before_quarantine()?;
    verify_stage(&stage)?;
    require_uncertified(directory)?;
    ensure!(inventory::inspect(directory)? == expected, CHANGED);
    let source = entry_c_string(location.directory.file_name().context(CHANGED)?)?;
    ensure!(
        super::candidate::same_current(root, &location.directory, &source, directory)?,
        CHANGED
    );
    let revision = directory.revision()?;
    let quarantine = entry_c_string(OsStr::new(&format!(
        "{QUARANTINE_PREFIX}{}",
        uuid::Uuid::new_v4().simple()
    )))?;
    let preserved_path = root.path().join(OsStr::from_bytes(quarantine.as_bytes()));
    fence()?;
    verify_stage(&stage)?;
    ensure!(directory.revision()? == revision, CHANGED);
    ensure!(
        root_mutation::rename_exclusive(root, &source, &quarantine, lifecycle)?,
        TARGET_OCCUPIED
    );
    let preserved = root.open_descendant(&preserved_path)?;
    ensure!(preserved.same_identity(directory), CHANGED);
    fence()?;
    tracing::warn!(path = %preserved_path.display(), message = QUARANTINED);
    // Drop closes the lifecycle descriptor; it never unlinks the quarantined entry.
    drop(stage);
    Ok(())
}

fn require_uncertified(directory: &PrivateDirectory) -> Result<()> {
    ensure!(
        super::directory_certificate::read(directory)?.is_none(),
        CERTIFICATE_PRESENT
    );
    Ok(())
}

pub(crate) fn verify_stage(stage: &PrivateFileLock) -> Result<()> {
    stage.verify()?;
    match stage.verify_retained_path() {
        RetainedPrivateFileVerification::Verified => Ok(()),
        RetainedPrivateFileVerification::IdentityLost => anyhow::bail!(CHANGED),
        RetainedPrivateFileVerification::Indeterminate(error) => Err(error),
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../test_support/integration/socket_namespace_unbound_fixture.rs"]
mod fixture;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/socket_namespace/socket_namespace_unbound_tests.rs"]
mod tests;
