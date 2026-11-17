#[cfg(any(test, feature = "adversarial-test-hooks"))]
use super::guard::SessionDurableMutationGuard;
use crate::namespace_lease::NamespaceLease;
use crate::paths::MAX_DURABLE_JSON_BYTES;
use crate::paths::identity::{DurableFileVersion, FileIdentity};
use crate::paths::private_file::PrivateFileEntry;
use anyhow::Result;
use serde::de::DeserializeOwned;
use std::path::Path;

pub(super) trait DurableRead {
    fn read_json_with_durable_version<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<(T, DurableFileVersion)>;
}

impl DurableRead for NamespaceLease {
    fn read_json_with_durable_version<T: DeserializeOwned>(
        &self,
        path: &Path,
    ) -> Result<(T, DurableFileVersion)> {
        let (value, _, version) = self.read_json_with_versions(path)?;
        Ok((value, version))
    }
}

#[cfg(any(test, feature = "adversarial-test-hooks"))]
pub(super) fn optional<T: DeserializeOwned>(
    guard: &SessionDurableMutationGuard<'_>,
    path: &Path,
) -> Result<Option<(T, DurableFileVersion)>> {
    Ok(guard
        .read_optional_with_versions(path)?
        .map(|(value, _, version)| (value, version)))
}

pub(super) fn capture(entry: &PrivateFileEntry) -> Result<DurableFileVersion> {
    let opened = entry.open_read(MAX_DURABLE_JSON_BYTES)?;
    let (version, live) = DurableFileVersion::capture_file_bounded(
        opened.file(),
        entry.path(),
        MAX_DURABLE_JSON_BYTES,
    )?;
    entry.verify_exact_identity(live, MAX_DURABLE_JSON_BYTES)?;
    Ok(version)
}

pub(super) fn from_live(
    entry: &PrivateFileEntry,
    expected: FileIdentity,
) -> Result<DurableFileVersion> {
    let opened = entry.open_read(MAX_DURABLE_JSON_BYTES)?;
    let (version, live) = DurableFileVersion::capture_file_bounded(
        opened.file(),
        entry.path(),
        MAX_DURABLE_JSON_BYTES,
    )?;
    anyhow::ensure!(live == expected, super::constants::PARTICIPANT_CONFLICT);
    entry.verify_exact_identity(live, MAX_DURABLE_JSON_BYTES)?;
    Ok(version)
}

pub(super) fn strict(
    entry: &PrivateFileEntry,
    expected: DurableFileVersion,
) -> Result<FileIdentity> {
    let opened = entry.open_read(MAX_DURABLE_JSON_BYTES)?;
    let live = expected.verify_file_bounded(opened.file(), entry.path(), MAX_DURABLE_JSON_BYTES)?;
    entry.verify_exact_identity(live, MAX_DURABLE_JSON_BYTES)?;
    Ok(live)
}

pub(super) fn content(
    entry: &PrivateFileEntry,
    expected: DurableFileVersion,
) -> Result<FileIdentity> {
    let opened = entry.open_read(MAX_DURABLE_JSON_BYTES)?;
    let live = expected.verify_content_version_bounded(
        opened.file(),
        entry.path(),
        MAX_DURABLE_JSON_BYTES,
    )?;
    entry.verify_exact_identity(live, MAX_DURABLE_JSON_BYTES)?;
    Ok(live)
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/durable_io.rs"]
mod integration_tests;
