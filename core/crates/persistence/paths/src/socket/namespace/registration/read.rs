use super::{NamespaceBinding, constants::*, record::Record};
use crate::paths::{
    directory::PrivateDirectory,
    private_file::{OpenedPrivateFile, PrivateFileEntry},
};
use anyhow::{Result, ensure};
use std::{io::Read, path::Path};

pub(super) fn entry(home: &Path) -> Result<(PrivateDirectory, PrivateFileEntry)> {
    let home = crate::paths::platform::platform_path(home);
    let directory = PrivateDirectory::open(&home)?;
    let entry =
        PrivateFileEntry::from_directory(directory.try_clone()?, &home.join(REGISTRATION_FILE))?;
    Ok((directory, entry))
}

pub(super) fn open_existing(
    home: &Path,
) -> Result<Option<(PrivateDirectory, PrivateFileEntry, OpenedPrivateFile)>> {
    // Absence is a separate initial observation. Once a home/entry has been observed,
    // later ENOENT represents lost proof and must not turn into permission to initialize.
    match std::fs::symlink_metadata(home) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
        Ok(_) => {}
    }
    let (directory, entry) = entry(home)?;
    let Some(identity) = entry.inspect(REGISTRATION_MAX_BYTES)? else {
        return Ok(None);
    };
    let opened = entry.open_read(REGISTRATION_MAX_BYTES)?;
    ensure!(opened.identity() == identity, INVALID_REGISTRATION);
    Ok(Some((directory, entry, opened)))
}

pub(super) fn verify_opened(
    directory: &PrivateDirectory,
    entry: &PrivateFileEntry,
    opened: &mut OpenedPrivateFile,
    expected: &NamespaceBinding,
) -> Result<()> {
    let mut bytes = Vec::new();
    opened
        .file_mut()
        .take(REGISTRATION_MAX_BYTES + READ_SENTINEL)
        .read_to_end(&mut bytes)?;
    ensure!(
        u64::try_from(bytes.len())? <= REGISTRATION_MAX_BYTES,
        INVALID_REGISTRATION
    );
    let record: Record = serde_json::from_slice(&bytes)?;
    record.validate(directory.durable_identity()?, expected)?;
    #[cfg(all(test, feature = "yolo-integration-tests"))]
    test_support::before_verification();
    opened.identity().verify_file_bounded(
        opened.file(),
        u64::try_from(bytes.len())?,
        entry.path(),
        REGISTRATION_MAX_BYTES,
    )?;
    entry.verify_current_unchanged(opened.identity(), REGISTRATION_MAX_BYTES)?;
    directory.verify_path()
}

pub(super) fn durable_identity(
    entry: &PrivateFileEntry,
    opened: &OpenedPrivateFile,
) -> Result<crate::paths::identity::DurableObjectIdentity> {
    let identity = opened
        .durable_version(entry.path(), REGISTRATION_MAX_BYTES)?
        .object_identity();
    entry.verify_current_unchanged(opened.identity(), REGISTRATION_MAX_BYTES)?;
    Ok(identity)
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../../test_support/integration/socket_namespace_registration_read_hook.rs"]
pub(super) mod test_support;
