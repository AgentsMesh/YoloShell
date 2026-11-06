use super::PtyCapabilityScanState;

impl PtyCapabilityScanState {
    pub(in crate::process) fn released_sweep_diagnostic_for_test(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        return Some(self.released.sweep_diagnostic_for_test());
        #[cfg(not(target_os = "macos"))]
        None
    }

    pub(in crate::process) fn liveness_for_test(&self) -> Option<String> {
        #[cfg(target_os = "macos")]
        return Some(format!("{:?}", self.liveness));
        #[cfg(not(target_os = "macos"))]
        None
    }

    pub(in crate::process) fn released_matched_lineages_for_test(&self) -> Vec<u64> {
        #[cfg(target_os = "macos")]
        return self.released.matched_lineages_for_test();
        #[cfg(not(target_os = "macos"))]
        Vec::new()
    }

    pub(in crate::process) fn released_pending_lineages_for_test(&self) -> Vec<u64> {
        #[cfg(target_os = "macos")]
        return self.released.pending_lineages_for_test();
        #[cfg(not(target_os = "macos"))]
        Vec::new()
    }

    pub(in crate::process) fn released_round_size_for_test(&self) -> usize {
        #[cfg(target_os = "macos")]
        return self.released.round_size_for_test();
        #[cfg(not(target_os = "macos"))]
        usize::default()
    }

    pub(in crate::process) fn released_holder_observation_for_test(
        &self,
    ) -> (Option<String>, usize, usize) {
        #[cfg(target_os = "macos")]
        return (
            Some(format!("{:?}", self.released.holder_observation.status)),
            self.released.holder_observation.holder_pid_count,
            self.released.holder_observation.matched_identity_count,
        );
        #[cfg(not(target_os = "macos"))]
        (None, usize::default(), usize::default())
    }
}
