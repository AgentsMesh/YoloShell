use super::*;
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;
use yolo_process_test_support::{
    ProcessIdentity as RuntimeProcessIdentity, ProcessIdentityObservation,
};
use yolo_resilience_support::secondary_custody_adversarial_observation::ActiveCustodyObservation;

const CUSTODY_INSPECTION_CONTEXT: &str = "inspect simulated power-loss PTY custody";
const MISSING_GUARDIAN_DETAIL: &str = "simulated power-loss Guardian is missing";
const UNEXPECTED_GUARDIAN_DETAIL: &str = "synthetic Guardian-free fixture contains PTY custody";
const GUARDIAN_BINDING_MISMATCH_DETAIL: &str =
    "simulated power-loss Guardian does not match its runtime";
const GUARDIAN_ALIAS_DETAIL: &str = "simulated power-loss Guardian identity is aliased";
const GUARDIAN_NOT_LIVE_DETAIL: &str = "simulated power-loss Guardian identity is not live";

pub(super) struct BoundCustody {
    terminal_id: Uuid,
    observation: ActiveCustodyObservation,
    recovery: Option<yolo_process_test_support::PtyCustodyRecoveryAuthority>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GuardianCandidate {
    terminal_id: Uuid,
    observation: ActiveCustodyObservation,
}

impl BoundCustody {
    fn bind(record: &RuntimeRecord, candidate: GuardianCandidate) -> Result<Self> {
        ensure!(
            candidate.terminal_id == record.terminal_id,
            GUARDIAN_BINDING_MISMATCH_DETAIL
        );
        ensure!(
            candidate.observation.matches_runtime(record),
            GUARDIAN_BINDING_MISMATCH_DETAIL
        );
        Ok(Self {
            terminal_id: candidate.terminal_id,
            observation: candidate.observation,
            recovery: None,
        })
    }

    pub(super) fn terminal_id(&self) -> Uuid {
        self.terminal_id
    }

    pub(super) fn identity(&self) -> RuntimeProcessIdentity {
        self.observation.guardian()
    }

    pub(super) fn direct_target(&self) -> Option<RuntimeProcessIdentity> {
        Some(self.identity())
    }

    pub(super) async fn recover(&mut self, paths: &Paths) -> Result<()> {
        ensure!(self.recovery.is_none(), GUARDIAN_BINDING_MISMATCH_DETAIL);
        self.recovery = Some(
            self.observation
                .recover_authority(paths, self.terminal_id)
                .await?,
        );
        Ok(())
    }

    pub(super) fn take_recovery(
        &mut self,
    ) -> Result<yolo_process_test_support::PtyCustodyRecoveryAuthority> {
        self.recovery
            .take()
            .context(GUARDIAN_BINDING_MISMATCH_DETAIL)
    }

    pub(super) fn confirm_active(&self, paths: &Paths) -> Result<()> {
        self.observation.confirm_unchanged(paths, self.terminal_id)?;
        ensure!(
            self.identity().observe() == ProcessIdentityObservation::Matching,
            GUARDIAN_NOT_LIVE_DETAIL
        );
        Ok(())
    }

    pub(super) fn confirm_retained(&self, paths: &Paths) -> Result<()> {
        self.observation
            .confirm_retained_unchanged(paths, self.terminal_id)
    }
}

impl GuardianCandidate {
    fn read(paths: &Paths, terminal_id: Uuid) -> Result<Self> {
        let path = paths.pty_custody(terminal_id);
        ensure!(
            path.try_exists().context(CUSTODY_INSPECTION_CONTEXT)?,
            "{MISSING_GUARDIAN_DETAIL}: {terminal_id}"
        );
        let observation = ActiveCustodyObservation::read(paths, terminal_id)
            .context(CUSTODY_INSPECTION_CONTEXT)?;
        Ok(Self {
            terminal_id,
            observation,
        })
    }
}

pub(super) fn confirm_absent(paths: &Paths, terminal_id: Uuid) -> Result<()> {
    ensure!(
        !paths
            .pty_custody(terminal_id)
            .try_exists()
            .context(CUSTODY_INSPECTION_CONTEXT)?,
        UNEXPECTED_GUARDIAN_DETAIL
    );
    Ok(())
}

pub(super) fn capture(
    paths: &Paths,
    records: &[BoundRuntimeRecord],
    expectation: GuardianExpectation,
) -> Result<Vec<BoundCustody>> {
    if expectation == GuardianExpectation::ExplicitlyAbsentFixture {
        for record in records {
            ensure!(
                !paths
                    .pty_custody(record.record().terminal_id)
                    .try_exists()
                    .context(CUSTODY_INSPECTION_CONTEXT)?,
                UNEXPECTED_GUARDIAN_DETAIL
            );
        }
        return Ok(Vec::new());
    }
    let candidates = records
        .iter()
        .map(|record| GuardianCandidate::read(paths, record.record().terminal_id))
        .collect::<Result<Vec<_>>>()?;
    bind_candidates(records, candidates)
}

fn bind_candidates(
    records: &[BoundRuntimeRecord],
    candidates: Vec<GuardianCandidate>,
) -> Result<Vec<BoundCustody>> {
    ensure!(records.len() == candidates.len(), MISSING_GUARDIAN_DETAIL);
    let mut targets = records
        .iter()
        .flat_map(|bound| [runtime_worker(bound), runtime_child(bound)])
        .collect::<HashSet<_>>();
    records
        .iter()
        .zip(candidates)
        .map(|(record, candidate)| {
            let bound = BoundCustody::bind(record.record(), candidate)?;
            ensure!(targets.insert(bound.identity()), GUARDIAN_ALIAS_DETAIL);
            ensure!(
                bound.identity().observe() == ProcessIdentityObservation::Matching,
                GUARDIAN_NOT_LIVE_DETAIL
            );
            Ok(bound)
        })
        .collect()
}

#[cfg(test)]
#[path = "power_loss_guardian_tests.rs"]
mod tests;
