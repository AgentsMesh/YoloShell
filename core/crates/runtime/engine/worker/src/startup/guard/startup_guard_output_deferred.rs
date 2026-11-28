use super::super::pty_fault_reaper::FaultReaperReservation;
use super::{STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT, StartupChildGuard, StartupOutputProgress};
use crate::process::{CapturedProcessTree, RootWaitAuthority};
use anyhow::{Context, Result};

impl StartupChildGuard {
    pub(crate) fn defer_tracked_output(
        &mut self,
        authority: RootWaitAuthority,
        tree: CapturedProcessTree,
        output: StartupOutputProgress,
        failure: Option<anyhow::Error>,
        reservation: FaultReaperReservation,
    ) -> Result<()> {
        let mut handoff = reservation.prepare_startup_handoff(authority, tree, output, failure);
        self.clear();
        handoff.accept();
        Err(anyhow::Error::new(
            super::super::termination_timing::CapabilitySettlementDeferred,
        ))
    }

    pub(crate) fn reserve_deferred_output(
        &mut self,
        supported: bool,
    ) -> Result<Option<FaultReaperReservation>> {
        if supported {
            return Ok(None);
        }
        self.fault_reaper
            .take()
            .map(Some)
            .context(STARTUP_FAULT_REAPER_UNAVAILABLE_CONTEXT)
    }
}
