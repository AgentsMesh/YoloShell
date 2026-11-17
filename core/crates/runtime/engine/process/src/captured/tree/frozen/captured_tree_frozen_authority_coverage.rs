impl CapturedProcessTree {
    #[cfg(target_os = "macos")]
    pub(super) fn commit_confirmed_bootstrap_global_exclusion_if_authorized(&self) {
        if self.premature_release_empty_closure_coverage()
            && let Some(proof) = self.pty_proof.as_ref()
        {
            proof.commit_confirmed_bootstrap_global_exclusion();
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn commit_confirmed_bootstrap_global_exclusion_if_authorized(&self) {}

    #[cfg(target_os = "macos")]
    fn pending_capability_coverage(&self) -> bool {
        self.writer_present_coverage()
            || self.bootstrap_exec_fence_coverage()
            || self.premature_release_fallback_coverage()
    }

    #[cfg(target_os = "macos")]
    pub(super) fn bootstrap_exec_fence_coverage(&self) -> bool {
        let coverage = self.discovery_epoch.coverage_is_complete()
            || self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::bootstrap_capability_coverage_is_retained);
        coverage
            && self.events.is_gated()
            && self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::bootstrap_exec_is_fenced)
    }

    #[cfg(not(target_os = "macos"))]
    fn pending_capability_coverage(&self) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    pub(super) fn writer_present_coverage(&self) -> bool {
        self.discovery_epoch.coverage_is_complete()
            && self.pty_scan.writer_is_present()
            && self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::has_transferable_descriptor)
    }

    #[cfg(target_os = "macos")]
    fn premature_release_fallback_coverage(&self) -> bool {
        self.discovery_epoch.coverage_is_complete()
            && self.events.is_gated()
            && self.pty_scan.uses_premature_release_fallback()
            && self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::bootstrap_exec_was_confirmed)
    }

    #[cfg(target_os = "macos")]
    fn premature_release_empty_closure_coverage(&self) -> bool {
        let root = self.live_root();
        self.capability_scan_pending
            && self.lineage_settled
            && !self.lineage_integrity_lost
            && !self.frozen_fork_reconciliation_required
            && !self.events.has_unresolved_fork()
            && self.captured_targets().is_empty()
            && self.retiring.iter().all(|identity| identity == &root)
            && self.root_exit.is_gone()
            && self.premature_release_fallback_coverage()
    }

    #[cfg(not(target_os = "macos"))]
    fn premature_release_empty_closure_coverage(&self) -> bool {
        false
    }

    #[cfg(not(target_os = "macos"))]
    fn writer_present_coverage(&self) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    fn empty_output_fence_event_authority_is_current(&mut self) -> bool {
        self.events.is_gated()
            && (!self.events.has_unresolved_fork() || self.events.exact_closure_isolated(&[]))
            && self.events.post_inventory_quiescent()
    }

    #[cfg(not(target_os = "macos"))]
    fn empty_output_fence_event_authority_is_current(&mut self) -> bool {
        true
    }

    #[cfg(target_os = "macos")]
    pub(super) fn empty_output_fence_can_reconcile_uncertainty(&mut self) -> bool {
        let root = self.live_root();
        // Kernel EOF, rather than a negative descriptor sweep, is the final
        // global exclusion proof for this empty closure.
        self.capability_scan_pending
            && self.lineage_settled
            && self.captured_targets().is_empty()
            && self.retiring.iter().all(|identity| identity == &root)
            && self.empty_output_fence_event_authority_is_current()
            && self.empty_output_fence_is_authorized()
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn empty_output_fence_can_reconcile_uncertainty(&mut self) -> bool {
        false
    }
}
