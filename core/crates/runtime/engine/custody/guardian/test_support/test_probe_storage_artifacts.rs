use super::{ENTRY_LIMIT_DETAIL, ENTRY_NAME_DETAIL, ENTRY_VERSION_CHANGED, guardian_id_from_name};
use crate::namespace_lease::NamespaceLease;
use crate::paths::constants::{MAX_DURABLE_JSON_BYTES, MAX_PRIVATE_DIRECTORY_ENTRIES};
use crate::paths::directory::PrivateDirectory;
use crate::paths::directory_stream::DirectoryStream;
use crate::paths::identity::FileIdentity;
use crate::paths::private_file::{
    PrivateFileEntry, is_atomic_storage_artifact_for, is_storage_artifact,
};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs::File;

const STORAGE_ARTIFACT_BINDING_DETAIL: &str =
    "Guardian custody probe storage artifact has no exact target";
const STORAGE_ARTIFACT_OPEN_DETAIL: &str = "Guardian custody probe storage artifact is unsafe";

pub(super) struct ProbeDirectoryEntries {
    pub(super) raw_names: BTreeSet<OsString>,
    pub(super) probe_names: Vec<OsString>,
    pub(super) artifact_names: Vec<OsString>,
}

pub(super) struct ObservedStorageArtifact {
    entry: PrivateFileEntry,
    identity: FileIdentity,
    file: File,
}

pub(super) fn scan_entries(
    namespace: &NamespaceLease,
    directory: &PrivateDirectory,
) -> Result<ProbeDirectoryEntries> {
    let raw_names = read_raw_names(namespace, directory)?;
    let mut probe_names = Vec::new();
    let mut artifact_names = Vec::new();
    for name in &raw_names {
        if is_storage_artifact(name) {
            artifact_names.push(name.clone());
        } else {
            guardian_id_from_name(name)?;
            probe_names.push(name.clone());
        }
    }
    for artifact in &artifact_names {
        ensure!(
            artifact_is_bound(artifact, &probe_names)?,
            STORAGE_ARTIFACT_BINDING_DETAIL
        );
    }
    Ok(ProbeDirectoryEntries {
        raw_names,
        probe_names,
        artifact_names,
    })
}

fn read_raw_names(
    namespace: &NamespaceLease,
    directory: &PrivateDirectory,
) -> Result<BTreeSet<OsString>> {
    namespace.verify()?;
    let mut stream = DirectoryStream::open(directory.descriptor())?;
    let mut names = BTreeSet::new();
    let mut entry_count = 0_usize;
    while let Some(name) = stream.next_name()? {
        entry_count = entry_count.checked_add(1).context(ENTRY_LIMIT_DETAIL)?;
        ensure!(
            entry_count <= MAX_PRIVATE_DIRECTORY_ENTRIES,
            ENTRY_LIMIT_DETAIL
        );
        ensure!(names.insert(name), ENTRY_NAME_DETAIL);
    }
    directory.verify_descriptor_path()?;
    namespace.verify()?;
    Ok(names)
}

fn artifact_is_bound(artifact: &OsStr, targets: &[OsString]) -> Result<bool> {
    for target in targets {
        if is_atomic_storage_artifact_for(artifact, target)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn observe_storage_artifacts(
    namespace: &NamespaceLease,
    directory: &PrivateDirectory,
    names: &[OsString],
) -> Result<Vec<ObservedStorageArtifact>> {
    names
        .iter()
        .map(|name| observe_storage_artifact(namespace, directory, name))
        .collect()
}

fn observe_storage_artifact(
    namespace: &NamespaceLease,
    directory: &PrivateDirectory,
    name: &OsStr,
) -> Result<ObservedStorageArtifact> {
    let path = directory.path().join(name);
    let entry = PrivateFileEntry::from_directory(directory.try_clone()?, &path)?;
    let opened = entry
        .open_retained(MAX_DURABLE_JSON_BYTES)
        .context(STORAGE_ARTIFACT_OPEN_DETAIL)?;
    directory.verify_descriptor_path()?;
    namespace.verify()?;
    let (file, identity) = opened.into_parts();
    Ok(ObservedStorageArtifact {
        entry,
        identity,
        file,
    })
}

pub(super) fn verify_stable(artifacts: &[ObservedStorageArtifact]) -> Result<()> {
    for artifact in artifacts {
        artifact
            .identity
            .verify_file_bounded(
                &artifact.file,
                artifact.identity.length(),
                artifact.entry.path(),
                MAX_DURABLE_JSON_BYTES,
            )
            .context(ENTRY_VERSION_CHANGED)?;
        artifact
            .entry
            .verify_retained_unchanged(artifact.identity, MAX_DURABLE_JSON_BYTES)
            .context(ENTRY_VERSION_CHANGED)?;
    }
    Ok(())
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/probe/test_probe_storage_artifact_tests.rs"]
mod tests;
