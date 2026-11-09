use super::*;
use anyhow::{Context, Result, ensure};
use yolo_process_test_support::ProcessIdentity as RuntimeProcessIdentity;

const CUSTODY_INSPECTION_CONTEXT: &str = "inspect unsupported simulated power-loss PTY custody";
const UNEXPECTED_CUSTODY_DETAIL: &str =
    "synthetic Guardian-free fixture contains unsupported PTY custody";
const REQUIRED_AUTHORITY_UNAVAILABLE: &str =
    "simulated power loss requires a supported PTY containment authority";

#[derive(Clone, Copy)]
pub(super) struct BoundCustody {
    terminal_id: Uuid,
    identity: RuntimeProcessIdentity,
}

impl BoundCustody {
    pub(super) fn terminal_id(self) -> Uuid {
        self.terminal_id
    }

    pub(super) fn direct_target(&self) -> Option<RuntimeProcessIdentity> {
        Some(self.identity)
    }

    pub(super) fn confirm_active(self, _paths: &Paths) -> Result<()> {
        Ok(())
    }

    pub(super) fn confirm_retained(self, _paths: &Paths) -> Result<()> {
        Ok(())
    }
}

pub(super) fn capture(
    paths: &Paths,
    records: &[BoundRuntimeRecord],
    expectation: GuardianExpectation,
) -> Result<Vec<BoundCustody>> {
    ensure!(
        expectation == GuardianExpectation::ExplicitlyAbsentFixture,
        REQUIRED_AUTHORITY_UNAVAILABLE
    );
    for bound in records {
        confirm_absent(paths, bound.record().terminal_id)?;
    }
    Ok(Vec::new())
}

pub(super) fn confirm_absent(paths: &Paths, terminal_id: Uuid) -> Result<()> {
    ensure!(
        !paths
            .pty_custody(terminal_id)
            .try_exists()
            .context(CUSTODY_INSPECTION_CONTEXT)?,
        "{UNEXPECTED_CUSTODY_DETAIL}: {terminal_id}",
    );
    Ok(())
}

#[cfg(test)]
#[path = "power_loss_guardians_unsupported_tests.rs"]
mod tests;
