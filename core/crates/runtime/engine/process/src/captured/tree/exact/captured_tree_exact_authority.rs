use super::CapturedProcessTree;
use super::frozen_authority::CapturedStoppedClosureAuthority;
use crate::process::LiveProcessIdentity;
use crate::process::captured_lineage::LineageObservationStatus;
use crate::process::captured_targets::ExactSigkillResolution;

impl CapturedProcessTree {
    pub(super) fn commit_exact_lineage_containment(&mut self, resolution: &ExactSigkillResolution) {
        if self.pty_proof.is_some() {
            return;
        }
        let versions = self.events.unresolved_fork_versions();
        let allowed = self
            .exact_containment_lineage
            .related_sources(&versions, resolution.target_lineages());
        self.exact_lineage_containment = self.events.exact_lineage_containment_authority(&allowed);
    }

    pub(super) fn acknowledge_exact_lineage_containment(
        &mut self,
        lineage_status: LineageObservationStatus,
        publication_quiescent: bool,
    ) -> bool {
        if self.pty_proof.is_some()
            || lineage_status != LineageObservationStatus::Complete
            || !publication_quiescent
            || !self.captured_targets().is_empty()
        {
            return false;
        }
        let Some(authority) = self.exact_lineage_containment.take() else {
            return false;
        };
        let acknowledged = self.events.acknowledge_exact_containment(&[], &authority);
        if !acknowledged {
            self.exact_lineage_containment = Some(authority);
        } else {
            self.exact_containment_lineage.clear();
        }
        acknowledged
    }

    #[cfg(target_os = "macos")]
    pub(super) fn exact_stopped_closure_can_kill(
        &mut self,
        authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        let discovery_covered = self.discovery_epoch.coverage_is_complete();
        let isolated = self.exact_stopped_closure_isolated(authority);
        let pty_authorized = self.exact_pty_closure_is_authorized(authority.closure());
        #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
        self.trace_exact_authority(
            super::exact_authority_diagnostic::ExactAuthorityLane::Global,
            authority,
            discovery_covered,
            isolated,
            pty_authorized,
        );
        discovery_covered && isolated && pty_authorized
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn exact_stopped_closure_can_kill(
        &mut self,
        _authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    pub(super) fn kernel_eof_stopped_closure_can_kill(
        &mut self,
        authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        self.released_scheduling_coverage()
            && self.empty_output_fence_is_authorized()
            && self.exact_stopped_closure_isolated(authority)
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn kernel_eof_stopped_closure_can_kill(
        &mut self,
        _authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    pub(super) fn exact_pty_closure_is_authorized(&self, closure: &[LiveProcessIdentity]) -> bool {
        self.writer_present_coverage()
            || (self
                .pty_proof
                .as_ref()
                .is_some_and(super::PtySessionProof::has_transferable_descriptor)
                && self.pty_scan.covers_exact_released_closure(closure))
    }

    #[cfg(target_os = "macos")]
    pub(super) fn incremental_stopped_closure_can_kill(
        &mut self,
        authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        let isolated = self.incremental_stopped_closure_isolated(authority);
        let pty_authorized = self.exact_pty_closure_is_authorized(authority.closure());
        #[cfg(all(feature = "adversarial-test-hooks", not(test)))]
        self.trace_exact_authority(
            super::exact_authority_diagnostic::ExactAuthorityLane::Incremental,
            authority,
            true,
            isolated,
            pty_authorized,
        );
        isolated && pty_authorized
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn incremental_stopped_closure_can_kill(
        &mut self,
        _authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        false
    }

    #[cfg(not(target_os = "macos"))]
    pub(super) fn exact_pty_closure_is_authorized(&self, _closure: &[LiveProcessIdentity]) -> bool {
        false
    }

    #[cfg(target_os = "macos")]
    fn exact_stopped_closure_isolated(
        &mut self,
        authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        self.stopped_closure_isolated(authority) && self.events.post_inventory_quiescent()
    }

    #[cfg(target_os = "macos")]
    fn incremental_stopped_closure_isolated(
        &mut self,
        authority: &CapturedStoppedClosureAuthority,
    ) -> bool {
        self.stopped_closure_isolated(authority)
            && self.events.post_inventory_event_read_quiescent()
    }

    #[cfg(target_os = "macos")]
    fn stopped_closure_isolated(&self, authority: &CapturedStoppedClosureAuthority) -> bool {
        let closure = authority.closure();
        let lineage_safe = if self.events.has_unresolved_fork() {
            self.events.exact_closure_isolated(closure)
        } else {
            !self.lineage_integrity_lost
        };
        !self.frozen_fork_reconciliation_required
            && self.lineage_settled
            && lineage_safe
            && authority.all_members_stopped()
            && self
                .captured_targets()
                .iter()
                .map(|(identity, _durable)| identity)
                .eq(closure)
            && self.events.is_gated()
    }
}

#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/exact/captured_tree_exact_containment_tests.rs"]
mod exact_containment_tests;
#[cfg(all(test, target_os = "macos"))]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/captured/tree/exact/captured_tree_exact_membership_tests.rs"]
mod exact_membership_tests;
