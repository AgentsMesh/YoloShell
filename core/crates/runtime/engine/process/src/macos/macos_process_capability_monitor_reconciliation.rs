use super::{
    MONITOR_ROOT_DETAIL, MacosProcessCapabilityManagerReader, MacosProcessCapabilityMonitor,
    lock_progress,
};
use crate::process::LiveProcessIdentity;
use anyhow::{Result, ensure};
use std::os::fd::OwnedFd;

impl MacosProcessCapabilityManagerReader {
    pub(crate) fn reconcile_armed_root_exit(&self) -> Result<()> {
        let monitor = self
            .monitor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(monitor) = monitor.as_ref() else {
            return Ok(());
        };
        monitor.reconcile_root_exit(&self.descriptor)
    }
}

impl MacosProcessCapabilityMonitor {
    pub(super) fn confirm_root_exit(
        &self,
        descriptor: &OwnedFd,
        root: &LiveProcessIdentity,
    ) -> Result<()> {
        ensure!(&self.root == root, MONITOR_ROOT_DETAIL);
        self.reconcile_root_exit(descriptor)
    }

    fn reconcile_root_exit(&self, descriptor: &OwnedFd) -> Result<()> {
        let mut progress = lock_progress(&self.shared);
        super::state::reconcile_confirmed_root_exit(descriptor, &self.root, &mut progress);
        if super::state::is_terminal(progress.observation) {
            self.queue.trigger_wake()?;
        }
        Ok(())
    }
}
