use super::CapturedProcessTree;
use super::ownership::SessionOwnership;
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
use crate::process::inventory::SessionOwnershipProof;
use crate::process::inventory_observation::CapabilityObservationMode;
use crate::process::{DescendantObservation, ProcessIdentity};
use anyhow::Result;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
use anyhow::bail;

#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/captured_tree_refresh_test_hook.rs"]
mod test_hook;

impl CapturedProcessTree {
    #[cfg(all(test, feature = "yolo-integration-tests"))]
    pub(crate) fn descendant_observation_is_settled(&self) -> bool {
        self.descendant_observation() == DescendantObservation::Settled
    }

    pub(crate) fn descendant_observation(&self) -> DescendantObservation {
        let root = self.scope.live_root();
        let capability_root =
            self.capability_scan_pending && self.retiring.iter().all(|identity| identity == &root);
        let proven_exited_root =
            self.root_exit_is_proven() && self.retiring.iter().all(|identity| identity == &root);
        let ownership_unproven = match self.ownership {
            SessionOwnership::Tracking => false,
            SessionOwnership::OwnershipFault => !self.root_exit.is_gone(),
            SessionOwnership::Retired => true,
        };
        if ownership_unproven
            || !self.lineage_settled
            || self.lineage_integrity_lost
            || (!self.retiring.is_empty() && !capability_root && !proven_exited_root)
        {
            return DescendantObservation::Indeterminate;
        }
        if self.capability_scan_pending {
            return DescendantObservation::CapabilityScanPending;
        }
        DescendantObservation::Settled
    }

    pub fn refresh(&mut self) -> Result<Vec<ProcessIdentity>> {
        self.refresh_with_capability(CapabilityObservationMode::Scan)
    }

    pub(super) fn refresh_for_kernel_eof(&mut self) -> Result<Vec<ProcessIdentity>> {
        self.refresh_with_capability(CapabilityObservationMode::KernelEofDeferred)
    }

    fn refresh_with_capability(
        &mut self,
        capability_mode: CapabilityObservationMode,
    ) -> Result<Vec<ProcessIdentity>> {
        #[cfg(any(test, feature = "adversarial-test-hooks"))]
        if let Some(detail) = self.injected_refresh_failure.take() {
            self.mark_ownership_fault();
            bail!(detail);
        }
        let proof = self.root_ownership_proof();
        if proof == SessionOwnershipProof::SessionRetired {
            return Ok(self.identities());
        }
        let invalidated = self.events.requires_reconciliation();
        self.restart_discovery_after_event(invalidated);
        let anchors = self
            .captured
            .keys()
            .chain(&self.retiring)
            .filter(|identity| !self.boundary.contains(identity))
            .cloned()
            .collect();
        #[cfg(any(test, feature = "runtime-component-tests"))]
        let _inventory = self.test_state.install_inventory();
        let observation = self.scope.observe_with_proof(
            &anchors,
            &self.boundary,
            &self.lineage,
            proof,
            self.pty_proof.as_ref(),
            &self.pty_scan,
            self.discovery_epoch.requested(),
            capability_mode,
        );
        #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
        if observation.is_ok()
            && let Some(parent) = test_hook::take_post_inventory_fork()
        {
            self.events.queue_test_fork(parent);
        }
        #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
        let observation = observation.and_then(|observation| {
            if test_hook::take_observation_failure() {
                bail!(test_hook::POST_OBSERVATION_FAILURE_DETAIL);
            }
            Ok(observation)
        });
        let result = observation.and_then(|observation| self.merge(observation));
        if let Err(error) = result {
            self.mark_ownership_fault();
            return Err(error);
        }
        Ok(self.identities())
    }

    pub fn refresh_if_changed(&mut self) -> Result<bool> {
        let event_reconciliation = self.events.requires_reconciliation();
        if !event_reconciliation && !self.capability_scan_pending {
            return Ok(false);
        }
        self.restart_discovery_after_event(event_reconciliation);
        if !event_reconciliation {
            self.refresh()?;
            return Ok(true);
        }
        let mut attempts = std::iter::repeat_n((), EVENT_RECONCILIATION_LIMIT).peekable();
        while attempts.next().is_some() {
            self.refresh()?;
            if !self.events.is_gated()
                || attempts.peek().is_none()
                || !self.events.requires_reconciliation()
            {
                break;
            }
        }
        Ok(true)
    }

    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub(crate) fn fail_next_refresh(&mut self, detail: &str) {
        self.injected_refresh_failure = Some(detail.to_owned());
    }

    #[cfg(any(all(test, feature = "yolo-integration-tests"), feature = "runtime-component-tests"))]
    pub fn invalidate_events(&mut self) {
        self.events.invalidate();
    }

    #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
    pub(crate) fn fail_next_event_read(&mut self) {
        self.events.fail_next_read();
    }

    #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
    pub(crate) fn fail_next_event_reconcile(&mut self) {
        self.events.fail_next_reconcile();
    }

    #[cfg(all(test, feature = "yolo-integration-tests"))]
    pub(crate) fn expire_event_audit(&mut self) {
        self.events.expire_audit();
    }

    #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
    pub(crate) fn set_capability_scan_pending_for_test(&mut self) {
        self.capability_scan_pending = true;
    }

    #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
    pub(crate) fn capability_scan_is_pending_for_test(&self) -> bool {
        self.capability_scan_pending
    }

    #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
    pub(in crate::process) fn queue_fork_after_next_inventory(
        &self,
        parent: crate::process::LiveProcessIdentity,
    ) -> test_hook::PostInventoryForkGuard {
        test_hook::install_post_inventory_fork(parent)
    }

    #[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
    fn fail_after_next_capability_observation(&self) -> test_hook::ObservationFailureGuard {
        test_hook::fail_after_next_observation()
    }
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/captured_tree_refresh_branch_tests.rs"]
mod branch_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/discovery/captured_tree_discovery_epoch_lineage_tests.rs"]
mod discovery_epoch_lineage_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/discovery/captured_tree_discovery_epoch_tests.rs"]
mod discovery_epoch_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/discovery/captured_tree_discovery_fallback_tests.rs"]
mod discovery_fallback_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/captured_tree_inventory_cache_tests.rs"]
mod inventory_cache_tests;
#[cfg(all(test, target_os = "macos", feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/captured/tree/post/captured_tree_post_inventory_fork_tests.rs"]
mod post_inventory_fork_tests;
