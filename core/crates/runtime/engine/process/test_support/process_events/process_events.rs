#[cfg(target_os = "macos")]
use super::MacosProcessEvents;
use super::{PROCESS_EVENT_AUDIT_INTERVAL, ProcessEvents};
#[cfg(target_os = "macos")]
use crate::process::LiveProcessIdentity;
use std::time::Instant;

impl ProcessEvents {
    pub(in crate::process) fn reconciliation_pending_for_test(&self) -> bool {
        self.pending_reconciliation
    }

    pub(in crate::process) fn unresolved_fork_lineages_for_test(&self) -> Vec<u64> {
        #[cfg(target_os = "macos")]
        return self.gate.as_ref().map_or_else(
            Vec::new,
            MacosProcessEvents::unresolved_fork_lineages_for_test,
        );
        #[cfg(not(target_os = "macos"))]
        Vec::new()
    }

    pub(in crate::process) fn invalidate(&mut self) {
        self.pending_reconciliation = true;
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn fail_next_read(&mut self) {
        self.gate.as_mut().unwrap().fail_next_read();
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn fail_next_reconcile(&mut self) {
        self.gate.as_mut().unwrap().fail_next_reconcile();
    }

    pub(in crate::process) fn expire_audit(&mut self) {
        self.reconciled_at = Instant::now() - PROCESS_EVENT_AUDIT_INTERVAL;
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn inject_unresolved_fork(&mut self, parent: LiveProcessIdentity) {
        self.invalidate_inventory_publication();
        self.pending_reconciliation = true;
        self.gate.as_mut().unwrap().inject_unresolved_fork(parent);
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn inject_gone_unresolved_fork(&mut self, parent: LiveProcessIdentity) {
        self.invalidate_inventory_publication();
        self.pending_reconciliation = true;
        self.gate
            .as_mut()
            .unwrap()
            .inject_gone_unresolved_fork(parent);
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn queue_test_fork(&mut self, parent: LiveProcessIdentity) {
        self.gate.as_mut().unwrap().queue_fork_test_read(parent);
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn queue_test_fork_exit(&mut self, parent: LiveProcessIdentity) {
        self.gate
            .as_mut()
            .unwrap()
            .queue_fork_exit_test_read(parent);
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn queue_test_exit(&mut self, parent: LiveProcessIdentity) {
        self.gate.as_mut().unwrap().queue_exit_test_read(parent);
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn queue_reconcile_test_fork_exit(
        &mut self,
        parent: LiveProcessIdentity,
    ) {
        self.gate
            .as_mut()
            .unwrap()
            .queue_reconcile_fork_exit_test_read(parent);
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn queue_quiescent_test_read(&mut self) {
        self.gate
            .as_mut()
            .unwrap()
            .queue_valid_test_read(usize::default());
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn disable_gate_for_test(&mut self) {
        self.gate = None;
    }
}
