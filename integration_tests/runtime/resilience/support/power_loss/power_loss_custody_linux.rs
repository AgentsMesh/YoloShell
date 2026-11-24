use super::*;
use anyhow::{Context, Result, ensure};
use yolo_process_test_support::ProcessIdentity as RuntimeProcessIdentity;
use yolo_runtime::paths::RetainedJsonFileObservation;
use yolo_runtime::pty_custody::{CustodyDescriptorSet, CustodyDurableRecord};

const CUSTODY_MISMATCH: &str = "Linux power-loss custody does not match its exact runtime";
const CUSTODY_REQUIRED: &str = "Linux power-loss runtime has no armed custody";
const UNEXPECTED_CUSTODY: &str = "synthetic containment fixture contains unexpected Host custody";

pub(super) struct BoundCustody {
    terminal_id: Uuid,
    observation: RetainedJsonFileObservation,
}

impl BoundCustody {
    pub(super) fn terminal_id(&self) -> Uuid {
        self.terminal_id
    }

    pub(super) fn direct_target(&self) -> Option<RuntimeProcessIdentity> {
        // Linux secondary custody is held by the fixture Host, which the Harness
        // stops after freezing every Worker and PTY tree. There is no Guardian.
        None
    }

    pub(super) fn confirm_active(&self, _paths: &Paths) -> Result<()> {
        self.observation.verify_unchanged()
    }

    pub(super) fn confirm_retained(&self, paths: &Paths) -> Result<()> {
        self.confirm_active(paths)
    }
}

pub(super) fn capture(
    paths: &Paths,
    records: &[BoundRuntimeRecord],
    expectation: GuardianExpectation,
) -> Result<Vec<BoundCustody>> {
    if expectation == GuardianExpectation::ExplicitlyAbsentFixture {
        for record in records {
            confirm_absent(paths, record.record().terminal_id)?;
        }
        return Ok(Vec::new());
    }
    records
        .iter()
        .map(|bound| {
            let runtime = bound.record();
            bound.verify_runtime_file()?;
            let (record, observation) = RetainedJsonFileObservation::bind::<CustodyDurableRecord>(
                &paths.pty_custody(runtime.terminal_id),
            )?;
            let lease = record.armed_lease().context(CUSTODY_REQUIRED)?;
            ensure!(
                record.is_valid()
                    && record.runtime.matches_record(runtime)
                    && lease.controller_generation == runtime.controller_generation
                    && lease.descriptor_set == CustodyDescriptorSet::LinuxContainment
                    && lease.guardian.is_none(),
                CUSTODY_MISMATCH
            );
            bound.verify_runtime_file()?;
            Ok(BoundCustody {
                terminal_id: runtime.terminal_id,
                observation,
            })
        })
        .collect()
}

pub(super) fn confirm_absent(paths: &Paths, terminal_id: Uuid) -> Result<()> {
    ensure!(
        !paths.pty_custody(terminal_id).try_exists()?,
        UNEXPECTED_CUSTODY
    );
    Ok(())
}
