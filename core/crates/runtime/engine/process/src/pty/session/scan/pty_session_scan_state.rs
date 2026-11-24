use super::{CapabilityScanAdvance, PtyCapabilityScanState};

impl CapabilityScanAdvance {
    pub(super) fn reset() -> Self {
        Self {
            next: PtyCapabilityScanState::default(),
            batch_consumed: false,
            evidence_observed: false,
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn released(next: super::macos_released::ReleasedScanState) -> Self {
        Self {
            next: PtyCapabilityScanState {
                released: next,
                ..PtyCapabilityScanState::default()
            },
            batch_consumed: false,
            evidence_observed: false,
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn macos_terminal(
        liveness: super::macos_liveness::MacosCapabilityLivenessState,
    ) -> Self {
        Self {
            next: PtyCapabilityScanState {
                liveness,
                ..PtyCapabilityScanState::default()
            },
            batch_consumed: false,
            evidence_observed: false,
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn with_macos_liveness(
        mut self,
        liveness: super::macos_liveness::MacosCapabilityLivenessState,
    ) -> Self {
        self.next.liveness = liveness;
        self
    }

    pub(super) fn with_consumed_batch(mut self, consumed: bool) -> Self {
        self.batch_consumed = consumed;
        self
    }

    #[cfg(target_os = "macos")]
    pub(super) fn with_observed_evidence(mut self, observed: bool) -> Self {
        self.evidence_observed = observed;
        self
    }

    pub(in crate::process) fn merge_scheduling_completion(
        &self,
        current: &PtyCapabilityScanState,
        retained: bool,
        observed: bool,
    ) -> bool {
        let begins_released_epoch = !current.proves_root_exit() && self.next.proves_root_exit();
        observed || (retained && !begins_released_epoch)
    }

    #[cfg(target_os = "linux")]
    pub(super) fn linux(next: super::linux_scan_state::LinuxScanState) -> Self {
        Self {
            next: PtyCapabilityScanState { linux: next },
            batch_consumed: false,
            evidence_observed: false,
        }
    }
}

impl PtyCapabilityScanState {
    #[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
    pub(in crate::process) fn adversarial_snapshot(&self) -> String {
        format!(
            "liveness={:?} {}",
            self.liveness,
            self.released.adversarial_snapshot(),
        )
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn writer_is_present(&self) -> bool {
        self.liveness == super::macos_liveness::MacosCapabilityLivenessState::WriterPresent
    }

    #[cfg(target_os = "macos")]
    pub(in crate::process) fn uses_premature_release_fallback(&self) -> bool {
        self.liveness == super::macos_liveness::MacosCapabilityLivenessState::PrematurelyReleased
    }

    #[cfg(all(test, target_os = "macos"))]
    pub(in crate::process) fn mark_released_needs_scan_for_test(&mut self) {
        self.liveness = super::macos_liveness::MacosCapabilityLivenessState::ReleasedNeedsScan;
    }

    pub(in crate::process) fn proves_root_exit(&self) -> bool {
        #[cfg(target_os = "macos")]
        {
            matches!(
                self.liveness,
                super::macos_liveness::MacosCapabilityLivenessState::ReleasedNeedsScan
                    | super::macos_liveness::MacosCapabilityLivenessState::ReleasedAfterRootExit
            ) || (self.uses_premature_release_fallback() && self.released.path_is_missing())
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    #[cfg(target_os = "macos")]
    pub fn covers_exact_released_closure(
        &self,
        closure: &[crate::process::LiveProcessIdentity],
    ) -> bool {
        self.released.covers_exact_closure(closure)
    }

    #[cfg(not(target_os = "macos"))]
    pub fn covers_exact_released_closure(
        &self,
        _closure: &[crate::process::LiveProcessIdentity],
    ) -> bool {
        false
    }

    pub(in crate::process) fn restart(&mut self) {
        #[cfg(target_os = "macos")]
        let liveness = match self.liveness {
            super::macos_liveness::MacosCapabilityLivenessState::ReleasedAfterRootExit => {
                super::macos_liveness::MacosCapabilityLivenessState::ReleasedNeedsScan
            }
            liveness => liveness,
        };
        #[cfg(target_os = "macos")]
        let released = self.released.restart_preserving_matches();
        *self = Self::default();
        #[cfg(target_os = "macos")]
        {
            self.liveness = liveness;
            self.released = released;
        }
    }

    pub(in crate::process) fn commit(&mut self, advance: CapabilityScanAdvance) -> bool {
        *self = advance.next;
        advance.batch_consumed
    }
}

#[cfg(any(test, feature = "runtime-component-tests"))]
#[path = "../../../../test_support/pty_session/pty_session_scan_state.rs"]
mod test_support;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/scan/pty_session_scan_state_tests.rs"]
mod tests;
