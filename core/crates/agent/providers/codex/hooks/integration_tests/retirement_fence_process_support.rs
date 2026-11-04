use super::retirement_test_constants::{MISSING_ADMISSION, MISSING_GONE_PROOF};
use super::retirement_test_support::RetirementFixture;
use crate::agent::hooks::{HookEnvelope, HookIdentity};
use crate::codex_hooks::{
    CodexAdmissionAuthority, CodexAdmissionStorage, CodexHookArtifacts, CodexHookIntake,
    CodexRetirementSeal,
};
use crate::process::{ExactProcessGone, ProcessIdentityObservation};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ProcessRaceContext {
    home: PathBuf,
    owner: HookIdentity,
    envelope: HookEnvelope,
}

pub(super) enum ProcessRaceRole {
    Producer,
    Sealer,
}

struct RecoveredRace {
    admission: CodexAdmissionAuthority,
    intake: CodexHookIntake,
}

impl ProcessRaceContext {
    pub(super) fn capture(fixture: &RetirementFixture) -> Self {
        Self {
            home: fixture.home().to_path_buf(),
            owner: fixture.owner().clone(),
            envelope: fixture.envelope(),
        }
    }

    pub(super) fn run(self, role: ProcessRaceRole) -> Result<()> {
        let recovered = self.recover()?;
        match role {
            ProcessRaceRole::Producer => recovered.intake.persist(self.envelope),
            ProcessRaceRole::Sealer => recovered.publish_seal(),
        }
    }

    fn recover(&self) -> Result<RecoveredRace> {
        let paths = crate::paths::Paths::new(&self.home);
        let artifacts = CodexHookArtifacts::new(
            &paths,
            self.owner.terminal_id,
            self.owner.runtime_epoch,
            self.owner.invocation_id,
        )?;
        let storage = CodexAdmissionStorage::for_identity(&artifacts, &self.owner)?;
        let admission = CodexAdmissionAuthority::recover(storage)?
            .ok_or_else(|| anyhow::anyhow!(MISSING_ADMISSION))?;
        let intake = CodexHookIntake::for_identity(artifacts.intake_directory(), &self.owner)?;
        Ok(RecoveredRace { admission, intake })
    }
}

impl RecoveredRace {
    fn publish_seal(&self) -> Result<()> {
        let process = crate::process::ProcessIdentity {
            pid: self.admission.record().request().identity.process.pid,
            start_time: self
                .admission
                .record()
                .request()
                .identity
                .process
                .start_time,
        };
        let gone = ExactProcessGone::capture_for_test(process, ProcessIdentityObservation::Gone)
            .context(MISSING_GONE_PROOF)?;
        let seal = CodexRetirementSeal::capture(&self.admission, gone)?;
        self.intake
            .acquire_writer_fence()?
            .publish_seal(&seal, &self.admission)?;
        Ok(())
    }
}
