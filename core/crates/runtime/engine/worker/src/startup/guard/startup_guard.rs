use super::pty_fault_reaper::FaultReaperReservation;
use super::pty_output_progress::{ExclusivePtyOutput, ReaderBackedPtyOutput};
use super::startup_scope::{StartupScopeMonitor, TrackedTree, refresh};
use super::startup_untracked_reaper::StartupUntrackedReaperReservation;
use crate::process::{LiveProcessIdentity, OwnedPtyChild, ProcessIdentity, RootWaitAuthority};
use anyhow::{Context, Result};
pub(crate) type PtyChild = OwnedPtyChild;
#[path = "startup_guard_constants.rs"]
mod constants;
pub(crate) use constants::*;
#[cfg(any(
    feature = "adversarial-test-hooks",
    feature = "startup-handoff-test-hooks"
))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/startup/startup_guard_adversarial.rs"]
mod adversarial;
#[path = "startup_guard_binding.rs"]
mod binding;
#[path = "startup_guard_bootstrap_failure.rs"]
mod bootstrap_failure;
#[path = "startup_guard_cleanup.rs"]
mod cleanup;
#[path = "startup_guard_construction.rs"]
mod construction;
#[path = "startup_guard_custody.rs"]
mod custody;
#[path = "startup_guard_custody_rollback.rs"]
mod custody_rollback;
#[path = "startup_guard_deferred.rs"]
mod deferred;
#[path = "startup_guard_deferred_untracked.rs"]
mod deferred_untracked;
#[path = "startup_guard_identity.rs"]
mod identity;
#[path = "startup_guard_output.rs"]
mod output;
#[path = "startup_guard_output_deferred.rs"]
mod output_deferred;
#[path = "startup_guard_terminal_containment.rs"]
mod terminal_containment;
#[path = "startup_guard_transfer.rs"]
mod transfer;
#[path = "startup_guard_untracked.rs"]
mod untracked;
#[path = "startup_guard_untracked_defer.rs"]
mod untracked_defer;
#[path = "startup_guard_untracked_progress.rs"]
mod untracked_progress;
#[path = "startup_guard_untracked_promotion.rs"]
mod untracked_promotion;

pub(crate) struct StartupChildGuard {
    pub(crate) child: Option<PtyChild>,
    pub(crate) authority: Option<RootWaitAuthority>,
    pub(crate) process_id: Option<u32>,
    pub(crate) identity: Option<ProcessIdentity>,
    pub(crate) live_identity: Option<LiveProcessIdentity>,
    pub(crate) tree: Option<TrackedTree>,
    pub(crate) monitor: Option<StartupScopeMonitor>,
    pub(crate) output_progress: Option<StartupOutputProgress>,
    pub(crate) untracked_reaper: Option<StartupUntrackedReaperReservation>,
    pub(crate) fault_reaper: Option<FaultReaperReservation>,
    pub(crate) unreleased_gate: Option<super::UnreleasedBootstrapGate>,
    pub(crate) untracked_gone_committed: bool,
}
pub(crate) enum StartupOutputProgress {
    Exclusive(ExclusivePtyOutput),
    ReaderBacked(ReaderBackedPtyOutput),
}
impl StartupChildGuard {
    #[cfg(test)]
    pub(crate) fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    pub(crate) fn identity(&self) -> Option<ProcessIdentity> {
        self.identity
    }

    pub(crate) fn refresh(&self) -> Result<()> {
        self.monitor
            .as_ref()
            .context(STARTUP_MONITOR_UNAVAILABLE_CONTEXT)?
            .check_failure()?;
        refresh(
            self.tree
                .as_ref()
                .context(STARTUP_TREE_UNAVAILABLE_CONTEXT)?,
        )
        .map(|_| ())
    }

    pub(crate) fn disarm_after_transfer(&mut self) {
        self.authority = None;
        self.identity = None;
        self.live_identity = None;
        self.process_id = None;
        self.untracked_reaper = None;
        self.unreleased_gate = None;
    }

    pub(crate) fn stop_monitor(&mut self) -> Result<()> {
        let result = self
            .monitor
            .as_mut()
            .map(StartupScopeMonitor::stop)
            .transpose();
        self.monitor = None;
        result.map(|_| ())
    }

    pub(crate) fn requires_cleanup(&self) -> bool {
        self.child.is_some()
            || self.authority.is_some()
            || self.identity.is_some()
            || self.output_progress.is_some()
            || self.fault_reaper.is_some()
            || self.unreleased_gate.is_some()
    }

    pub(crate) fn mark_owner_master_released(&mut self) {
        if let Some(StartupOutputProgress::Exclusive(output)) = self.output_progress.as_mut() {
            output.mark_owner_master_released();
        }
    }
}
impl Drop for StartupChildGuard {
    fn drop(&mut self) {
        if self.requires_cleanup() {
            terminal_containment::contain_drop_cleanup(|| self.defer_drop_cleanup());
        }
    }
}
#[cfg(test)]
use terminal_containment::contain_drop_cleanup;

#[cfg(all(test, target_os = "macos", feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_bootstrap_fence_cleanup_tests.rs"]
mod bootstrap_fence_cleanup_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_cleanup_tests.rs"]
mod cleanup_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_drop_adversarial_tests.rs"]
mod drop_adversarial_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_error_containment_tests.rs"]
mod error_containment_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_handoff_containment_tests.rs"]
mod handoff_containment_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_identity_tests.rs"]
mod identity_tests;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/startup/startup_guard_tests.rs"]
#[cfg(test)]
mod integration_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_panic_payload_tests.rs"]
mod panic_payload_tests;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/integration/startup_guard/startup_guard_process_assertions.rs"]
mod process_assertions;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/unit/startup/startup_guard_output_support.rs"]
mod test_output;
#[cfg(all(test, feature = "yolo-test-support"))]
#[path = "../../../test_support/startup_guard.rs"]
#[cfg(test)]
pub(crate) mod tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_guard_trace_containment_tests.rs"]
mod trace_containment_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_transfer_tests.rs"]
mod transfer_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_untracked_cleanup_tests.rs"]
mod untracked_cleanup_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/startup/startup_untracked_handoff_tests.rs"]
mod untracked_handoff_tests;
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-startup"))]
#[path = "../../../unit_tests/startup/startup_untracked_identity_matrix.rs"]
mod untracked_identity_matrix;
