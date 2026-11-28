use super::{StartupChildGuard, StartupOutputProgress};
use crate::pty_custody::{CustodyHolderIdentity, ExactCustodyReleasedAck};
use crate::worker::startup::startup_custody_transfer::StartupCustodyTransferFence;
use anyhow::{Result, bail};
use std::sync::Arc;

pub(crate) const STARTUP_EXCLUSIVE_OUTPUT_REQUIRED_DETAIL: &str =
    "pre-activation PTY custody rollback requires exclusive startup output";

impl StartupChildGuard {
    pub(crate) fn bind_custody_transfer_fence(
        &mut self,
        fence: Arc<StartupCustodyTransferFence>,
    ) -> Result<()> {
        self.exclusive_output()?.bind_custody_transfer_fence(fence)
    }

    pub(crate) fn require_host_release(&mut self, holder: CustodyHolderIdentity) -> Result<()> {
        self.exclusive_output()?.require_host_release(holder)
    }

    pub(crate) fn close_local_aliases_for_host_release(&mut self) -> Result<()> {
        self.exclusive_output()?
            .close_local_aliases_for_host_release()
    }

    pub(crate) fn prove_host_release(&mut self, exact: ExactCustodyReleasedAck) -> Result<()> {
        self.exclusive_output()?.prove_host_release(exact)
    }

    pub(crate) fn exclusive_output(
        &mut self,
    ) -> Result<&mut crate::worker::pty_output_progress::ExclusivePtyOutput> {
        let Some(StartupOutputProgress::Exclusive(output)) = self.output_progress.as_mut() else {
            bail!(STARTUP_EXCLUSIVE_OUTPUT_REQUIRED_DETAIL);
        };
        Ok(output)
    }
}
