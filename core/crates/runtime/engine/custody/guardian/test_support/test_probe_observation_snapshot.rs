use super::storage_artifacts::ObservedStorageArtifact;
use super::*;
use crate::paths::RetainedJsonFileObservation;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
use crate::paths::private_file::PrivateFileEntry;
use std::ffi::OsString;
use std::sync::Arc;
use std::time::Duration;

pub(super) struct ObservedProbe {
    pub(super) name: OsString,
    pub(super) identity: FileIdentity,
    pub(super) value: GuardianCustodyProbe,
    pub(super) observation: RetainedJsonFileObservation,
}

pub(super) fn read_probe<F>(
    namespace: &NamespaceLease,
    directory: &PrivateDirectory,
    name: &OsStr,
    observer: &mut F,
) -> Result<ObservedProbe>
where
    F: FnMut(ObservationBoundary) -> Result<()>,
{
    let guardian_id = guardian_id_from_name(name)?;
    let path = directory.path().join(name);
    let entry = Arc::new(PrivateFileEntry::from_directory(
        directory.try_clone()?,
        &path,
    )?);
    let (probe, observation): (GuardianCustodyProbe, _) =
        RetainedJsonFileObservation::read_entry_guarded_with_shared_lock(
            entry,
            || {
                observer(ObservationBoundary::AfterEntryOpen)?;
                directory.verify_descriptor_path()?;
                namespace.verify()
            },
            Duration::ZERO,
            crate::custody_guardian::settlement::POLL_INTERVAL,
        )
        .context(ENTRY_VERSION_CHANGED)?;
    observer(ObservationBoundary::AfterEntryRead)?;
    ensure!(
        probe.binding.schema_version == PROBE_SCHEMA_VERSION
            && probe.binding.guardian_id == guardian_id,
        ENTRY_NAME_DETAIL
    );
    Ok(ObservedProbe {
        name: name.to_owned(),
        identity: observation.identity(),
        value: probe,
        observation,
    })
}

pub(super) fn verify_stable(
    namespace: &NamespaceLease,
    session_directory: &PrivateDirectory,
    session_revision: crate::paths::directory::DirectoryRevision,
    directory: &PrivateDirectory,
    revision: crate::paths::directory::DirectoryRevision,
    probes: &[ObservedProbe],
    artifacts: &[ObservedStorageArtifact],
) -> Result<()> {
    for probe in probes {
        let path = directory.path().join(&probe.name);
        probe
            .observation
            .verify_unchanged()
            .context(ENTRY_VERSION_CHANGED)?;
        let entry = PrivateFileEntry::from_directory(directory.try_clone()?, &path)?;
        entry
            .verify_retained_unchanged(probe.identity, MAX_DURABLE_JSON_BYTES)
            .context(ENTRY_VERSION_CHANGED)?;
    }
    super::storage_artifacts::verify_stable(artifacts)?;
    ensure!(
        directory.revision()? == revision,
        DIRECTORY_REVISION_CHANGED
    );
    ensure!(
        session_directory.revision()? == session_revision,
        SESSION_REVISION_CHANGED
    );
    directory.verify_descriptor_path()?;
    session_directory.verify_descriptor_path()?;
    namespace.verify()
}
