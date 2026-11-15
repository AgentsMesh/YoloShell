use super::process_events::{ExactLineageContainmentAuthority, ProcessEvents};
use super::pty_session_proof::{CapabilityScanActivity, PtyCapabilityScanState};
use super::signal_authority::{DestructiveSessionAccess, DestructiveSessionAuthority};
use super::{LiveProcessIdentity, ProcessIdentity, PtyProcessScope, PtySessionProof};
use super::{captured_lineage::CapturedLineage, captured_targets};
use std::collections::{HashMap, HashSet};
#[cfg(all(feature = "adversarial-test-hooks", not(test)))]
#[path = "../../test_support/captured_tree/captured_tree_adversarial.rs"]
mod adversarial;
#[path = "tree/captured_tree_bootstrap_failure.rs"]
mod bootstrap_failure;
#[path = "tree/captured_tree_capture.rs"]
mod capture;
#[cfg(all(test, target_os = "macos"))]
pub(super) use capture::BOUND_PROOF_DETAIL;
#[path = "tree/captured_tree_compact.rs"]
mod compact;
#[path = "tree/discovery/captured_tree_discovery.rs"]
mod discovery;
#[cfg(target_os = "macos")]
#[path = "tree/discovery/captured_tree_discovery_fallback.rs"]
mod discovery_fallback;
#[path = "tree/discovery/captured_tree_discovery_publication.rs"]
mod discovery_publication;
#[path = "tree/exact/captured_tree_exact_authority.rs"]
mod exact_authority;
#[cfg(all(feature = "adversarial-test-hooks", not(test), target_os = "macos"))]
#[path = "tree/exact/captured_tree_exact_authority_diagnostic.rs"]
mod exact_authority_diagnostic;
#[path = "tree/exact/captured_tree_exact_lineage.rs"]
mod exact_lineage;
#[path = "tree/force/captured_tree_force.rs"]
mod force;
#[path = "tree/freeze/captured_tree_freeze.rs"]
mod freeze;
#[path = "tree/frozen/captured_tree_frozen_authority.rs"]
mod frozen_authority;
#[path = "tree/frozen/captured_tree_frozen_discovery.rs"]
mod frozen_discovery;
#[path = "tree/gone/captured_tree_gone.rs"]
mod gone;
#[path = "tree/captured_tree_kill.rs"]
mod termination;
pub(super) use termination::ExactClosureKill;
#[path = "tree/captured_tree_merge.rs"]
mod merge;
#[path = "tree/captured_tree_ownership.rs"]
mod ownership;
#[path = "tree/captured_tree_progress.rs"]
mod progress;
#[path = "tree/captured_tree_refresh.rs"]
mod refresh;
#[cfg(any(test, feature = "adversarial-test-hooks"))]
#[path = "tree/captured_tree_resilience.rs"]
mod resilience;
#[path = "tree/captured_tree_root_exit.rs"]
mod root_exit;
#[path = "tree/captured_tree_signal.rs"]
mod signal;
#[cfg(all(any(test, feature = "runtime-component-tests"), feature = "yolo-test-support"))]
#[path = "../../test_support/captured_tree/captured_tree_state.rs"]
mod state_test_support;
#[path = "tree/captured_tree_survivor.rs"]
mod survivor;
#[path = "tree/captured_tree_transition.rs"]
mod transition;
include!("tree/captured_tree_api.rs");
use ownership::SessionOwnership;
pub struct CapturedProcessTree {
    scope: PtyProcessScope,
    captured: HashMap<LiveProcessIdentity, ProcessIdentity>,
    boundary: super::controller_boundary::ProcessDiscoveryBoundary,
    retiring: HashSet<LiveProcessIdentity>,
    lineage: CapturedLineage,
    events: ProcessEvents,
    pty_proof: Option<PtySessionProof>,
    pty_scan: PtyCapabilityScanState,
    lineage_settled: bool,
    lineage_integrity_lost: bool,
    capability_scan_pending: bool,
    capability_scheduling_complete: bool,
    capability_scan_activity: CapabilityScanActivity,
    discovery_epoch: discovery::DiscoveryScanEpoch,
    discovery_authoritative_empty: bool,
    discovery_publication_audit: discovery_publication::DiscoveryPublicationAudit,
    frozen_fork_reconciliation_required: bool,
    exact_containment_lineage: exact_lineage::ExactContainmentLineage,
    exact_lineage_containment: Option<ExactLineageContainmentAuthority>,
    ownership: SessionOwnership,
    root_exit: root_exit::RootExitState,
    destructive: DestructiveSessionAccess,
    pty_master_released: bool,
    explicit_bootstrap_failure: bool,
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    injected_refresh_failure: Option<String>,
    #[cfg(target_os = "macos")]
    pty_local_alias_release_prepared: bool,
    closure_version: ProcessClosureVersion,
    #[cfg(any(test, feature = "runtime-component-tests"))]
    test_state: state_test_support::CapturedTreeTestState,
}
impl CapturedProcessTree {
    pub fn root(&self) -> ProcessIdentity {
        self.scope.root()
    }
    pub fn live_root(&self) -> LiveProcessIdentity {
        self.scope.live_root()
    }
    pub(super) fn destructive_authority(
        &self,
    ) -> std::result::Result<&DestructiveSessionAuthority, super::DestructiveSessionUnavailable>
    {
        let authority = self.destructive.require()?;
        if self
            .pty_proof
            .as_ref()
            .is_some_and(|proof| !proof.has_required_pty_object_lease())
        {
            return Err(super::DestructiveSessionUnavailable::new());
        }
        Ok(authority)
    }
    pub(crate) fn requires_pty_object_lease(&self) -> bool {
        super::pty_session_proof::requires_pty_object_lease(self.pty_proof.as_ref())
    }
    pub fn identities(&self) -> Vec<ProcessIdentity> {
        captured_targets::durable(self.sorted(self.captured.iter().filter_map(
            |(live, durable)| (!self.boundary.contains(live)).then_some((live.clone(), *durable)),
        )))
    }
    pub fn mark_ownership_fault(&mut self) {
        if self.ownership != ownership::SessionOwnership::Retired {
            self.ownership = ownership::SessionOwnership::OwnershipFault;
            self.capability_scheduling_complete = false;
        }
    }
    fn retire_session(&mut self) {
        self.ownership = ownership::SessionOwnership::Retired;
        self.capability_scheduling_complete = false;
    }
    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn retire_session_for_test(&mut self) {
        self.retire_session();
    }
    pub fn session_is_retired(&self) -> bool {
        self.ownership == ownership::SessionOwnership::Retired
    }
    pub(super) fn pty_master_released(&self) -> bool {
        self.pty_master_released
    }
    pub(super) fn mark_pty_master_released(
        &mut self,
        proof: crate::process::PtyMasterAliasesClosed,
    ) -> anyhow::Result<()> {
        let valid = self
            .pty_proof
            .as_ref()
            .is_some_and(|expected| expected.validates_master_aliases_closed(&proof));
        #[cfg(any(test, feature = "runtime-component-tests"))]
        let valid = valid || (self.pty_proof.is_none() && proof.is_scripted());
        anyhow::ensure!(
            valid,
            super::constants::PTY_MASTER_ALIAS_PROOF_MISMATCH_DETAIL
        );
        self.pty_master_released = true;
        Ok(())
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    #[cfg(any(test, feature = "adversarial-test-hooks"))]
    pub fn retiring_count(&self) -> usize {
        self.retiring.len()
    }
    #[cfg(all(test, target_os = "macos"))]
    pub(crate) fn events_are_gated_for_test(&self) -> bool {
        self.events.is_gated()
    }
    fn sorted(
        &self,
        identities: impl Iterator<Item = (LiveProcessIdentity, ProcessIdentity)>,
    ) -> Vec<(LiveProcessIdentity, ProcessIdentity)> {
        let root = self.scope.live_root();
        captured_targets::sorted(&root, identities)
    }
}
