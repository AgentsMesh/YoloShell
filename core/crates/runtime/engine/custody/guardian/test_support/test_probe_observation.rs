use super::*;
use crate::paths::directory::PrivateDirectory;
use crate::paths::identity::FileIdentity;
use crate::pty_custody::CustodyHolderObservation;
use std::ffi::OsStr;

const ENTRY_LIMIT_DETAIL: &str = "Guardian custody probe entry limit exceeded";
const ENTRY_NAME_DETAIL: &str = "Guardian custody probe entry name is invalid";
const ENTRY_SET_CHANGED: &str = "Guardian custody probe entry set changed";
const ENTRY_VERSION_CHANGED: &str = "Guardian custody probe entry version changed";
const DIRECTORY_REVISION_CHANGED: &str = "Guardian custody probe directory changed";
const SESSION_REVISION_CHANGED: &str = "Guardian custody probe session directory changed";
const INDETERMINATE_GUARDIAN: &str = "Guardian custody probe liveness is indeterminate";
const MULTIPLE_LIVE_GUARDIANS: &str = "session has more than one live PTY custody Guardian";
const LIVE_GUARDIAN_REQUIRED: &str = "session has no exact live PTY custody Guardian";

#[path = "test_probe_exact_observation.rs"]
mod exact_observation;
pub use exact_observation::ExactProbeObservation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ObservationBoundary {
    AfterFirstScan,
    AfterEntryOpen,
    AfterEntryRead,
    BeforeSecondScan,
    BeforeStableVerification,
}

#[path = "test_probe_observation_snapshot.rs"]
mod snapshot;
use snapshot::{ObservedProbe, read_probe, verify_stable};
#[path = "test_probe_storage_artifacts.rs"]
mod storage_artifacts;
use storage_artifacts::{observe_storage_artifacts, scan_entries};

pub(crate) fn read_exact_session(
    namespace: &NamespaceLease,
    paths: &Paths,
    record: &CustodyDurableRecord,
    identity: FileIdentity,
) -> Result<PtyMasterFdCustodyProbe> {
    read_exact_session_with(namespace, paths, record, identity, |_| Ok(()))
        .map(|value| value.custody())
}

pub fn read_exact_observation(
    namespace: &NamespaceLease,
    paths: &Paths,
    record: &CustodyDurableRecord,
    identity: FileIdentity,
) -> Result<ExactProbeObservation> {
    read_exact_session_with(namespace, paths, record, identity, |_| Ok(()))
}

#[cfg(test)]
pub(super) fn read_exact_session_observing<F>(
    namespace: &NamespaceLease,
    paths: &Paths,
    record: &CustodyDurableRecord,
    identity: FileIdentity,
    observer: F,
) -> Result<PtyMasterFdCustodyProbe>
where
    F: FnMut(ObservationBoundary) -> Result<()>,
{
    read_exact_session_with(namespace, paths, record, identity, observer)
        .map(|value| value.custody())
}

fn read_exact_session_with<F>(
    namespace: &NamespaceLease,
    paths: &Paths,
    record: &CustodyDurableRecord,
    identity: FileIdentity,
    mut observer: F,
) -> Result<ExactProbeObservation>
where
    F: FnMut(ObservationBoundary) -> Result<()>,
{
    let session_directory = namespace.directory(&paths.session_dir(record.runtime.terminal_id))?;
    let session_revision = session_directory.revision()?;
    let directory = namespace.directory(&probe_directory(paths, record.runtime.terminal_id))?;
    let revision = directory.revision()?;
    let entries = scan_entries(namespace, &directory)?;
    observer(ObservationBoundary::AfterFirstScan)?;
    let artifacts = observe_storage_artifacts(namespace, &directory, &entries.artifact_names)?;
    let mut probes = Vec::with_capacity(entries.probe_names.len());
    for name in &entries.probe_names {
        probes.push(read_probe(namespace, &directory, name, &mut observer)?);
    }
    observer(ObservationBoundary::BeforeSecondScan)?;
    let current_entries = scan_entries(namespace, &directory)?;
    ensure!(
        entries.raw_names == current_entries.raw_names,
        ENTRY_SET_CHANGED
    );
    let exact = select_exact_live(record, identity, &probes)?;
    observer(ObservationBoundary::BeforeStableVerification)?;
    verify_stable(
        namespace,
        &session_directory,
        session_revision,
        &directory,
        revision,
        &probes,
        &artifacts,
    )?;
    confirm_durable_unchanged(namespace, paths, record, identity)?;
    Ok(ExactProbeObservation::new(
        exact,
        session_revision,
        revision,
        &probes,
    ))
}

fn guardian_id_from_name(name: &OsStr) -> Result<Uuid> {
    let text = name.to_str().context(ENTRY_NAME_DETAIL)?;
    let stem = text
        .strip_suffix(PROBE_FILE_SUFFIX)
        .context(ENTRY_NAME_DETAIL)?;
    let guardian_id = Uuid::parse_str(stem).context(ENTRY_NAME_DETAIL)?;
    ensure!(
        text == format!("{guardian_id}{PROBE_FILE_SUFFIX}"),
        ENTRY_NAME_DETAIL
    );
    Ok(guardian_id)
}

fn select_exact_live(
    record: &CustodyDurableRecord,
    identity: FileIdentity,
    probes: &[ObservedProbe],
) -> Result<PtyMasterFdCustodyProbe> {
    let mut exact = None;
    for probe in probes {
        match probe.value.binding.guardian_process.observation() {
            CustodyHolderObservation::ProvenGone => continue,
            CustodyHolderObservation::Indeterminate => anyhow::bail!(INDETERMINATE_GUARDIAN),
            CustodyHolderObservation::Current => {}
        }
        ensure!(exact.is_none(), MULTIPLE_LIVE_GUARDIANS);
        let guardian = probe
            .value
            .binding
            .matches(record, identity)
            .context(MULTIPLE_LIVE_GUARDIANS)?;
        ensure!(
            probe
                .value
                .custody
                .process_holds_pty_object(guardian.process().pid)?,
            EXACT_DESCRIPTOR_REQUIRED
        );
        exact = Some(probe.value.custody.clone());
    }
    exact.context(LIVE_GUARDIAN_REQUIRED)
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/probe/test_probe_observation_adversarial_tests.rs"]
mod adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/probe/test_probe_observation_lock_tests.rs"]
mod lock_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "test_probe_observation_record_support.rs"]
mod test_record_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "test_probe_observation_test_support.rs"]
mod test_support;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/probe/test_probe_observation_tests.rs"]
mod tests;
