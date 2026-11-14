use super::snapshot::ObservedProbe;
use crate::paths::RetainedJsonFileObservation;
use crate::paths::directory::DirectoryRevision;
use crate::paths::identity::FileIdentity;
use crate::process::PtyMasterFdCustodyProbe;
use anyhow::{Context, Result, ensure};
use std::ffi::OsString;

const RETAINED_PROBE_CHANGED: &str = "retained Guardian custody probe changed";

#[derive(Clone, Debug)]
struct RetainedProbeEntry {
    name: OsString,
    identity: FileIdentity,
    _observation: RetainedJsonFileObservation,
}

impl PartialEq for RetainedProbeEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.identity == other.identity
    }
}

impl Eq for RetainedProbeEntry {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbeObservation {
    custody: PtyMasterFdCustodyProbe,
    session_revision: DirectoryRevision,
    directory_revision: DirectoryRevision,
    entries: Vec<RetainedProbeEntry>,
}

impl ExactProbeObservation {
    pub(super) fn new(
        custody: PtyMasterFdCustodyProbe,
        session_revision: DirectoryRevision,
        directory_revision: DirectoryRevision,
        probes: &[ObservedProbe],
    ) -> Self {
        let entries = probes
            .iter()
            .map(|probe| RetainedProbeEntry {
                name: probe.name.clone(),
                identity: probe.identity,
                _observation: probe.observation.clone(),
            })
            .collect();
        Self {
            custody,
            session_revision,
            directory_revision,
            entries,
        }
    }

    pub fn custody(&self) -> PtyMasterFdCustodyProbe {
        self.custody.clone()
    }

    pub fn confirm_retained_unchanged(&self) -> Result<()> {
        for entry in &self.entries {
            entry
                ._observation
                .verify_unchanged()
                .context(RETAINED_PROBE_CHANGED)?;
            ensure!(
                entry._observation.identity() == entry.identity,
                RETAINED_PROBE_CHANGED
            );
        }
        Ok(())
    }
}
