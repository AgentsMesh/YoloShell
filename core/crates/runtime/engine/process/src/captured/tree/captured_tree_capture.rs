use super::{CapturedProcessTree, SessionOwnership, compact, discovery, root_exit};
use crate::process::captured_lineage::CapturedLineage;
use crate::process::process_events::ProcessEvents;
use crate::process::pty_session_proof::PtyCapabilityScanState;
use crate::process::signal_authority::DestructiveSessionAccess;
use crate::process::{
    LiveProcessIdentity, ProcessIdentity, PtyProcessScope, PtySessionProof,
    WaitableChildSpawnBoundary,
};
use anyhow::{Result, ensure};
use std::collections::{HashMap, HashSet};

#[cfg(target_os = "macos")]
pub(in crate::process) const BOUND_PROOF_DETAIL: &str =
    "spawned PTY proof is already bound to process capability";

impl CapturedProcessTree {
    pub fn capture(root: ProcessIdentity) -> Result<Self> {
        Self::capture_with_scope(root, None, true, Vec::new(), ProcessEvents::new)
    }

    pub(crate) fn capture_lineage(root: ProcessIdentity) -> Result<Self> {
        Self::capture_with_scope(root, None, false, Vec::new(), ProcessEvents::new)
    }

    pub(crate) fn capture_pty(root: ProcessIdentity, proof: PtySessionProof) -> Result<Self> {
        Self::capture_with_scope(root, Some(proof), true, Vec::new(), ProcessEvents::new)
    }

    pub(crate) fn capture_lineage_excluding(
        root: ProcessIdentity,
        excluded: Vec<ProcessIdentity>,
    ) -> Result<Self> {
        Self::capture_with_scope(root, None, false, excluded, ProcessEvents::new)
    }

    pub(crate) fn capture_pty_excluding(
        root: ProcessIdentity,
        proof: PtySessionProof,
        excluded: Vec<ProcessIdentity>,
    ) -> Result<Self> {
        Self::capture_with_scope(root, Some(proof), true, excluded, ProcessEvents::new)
    }

    pub fn capture_pty_in_spawn_boundary(
        root: ProcessIdentity,
        proof: PtySessionProof,
        boundary: &WaitableChildSpawnBoundary<'_>,
    ) -> Result<Self> {
        #[cfg(target_os = "macos")]
        ensure!(!proof.has_transferable_descriptor(), BOUND_PROOF_DETAIL);
        Self::capture_with_scope(root, Some(proof), true, Vec::new(), |root| {
            ProcessEvents::new_in_spawn_boundary(root, boundary)
        })
    }

    fn capture_with_scope(
        root: ProcessIdentity,
        pty_proof: Option<PtySessionProof>,
        include_owned_session: bool,
        excluded: Vec<ProcessIdentity>,
        initialize_events: impl FnOnce(LiveProcessIdentity) -> ProcessEvents,
    ) -> Result<Self> {
        let scope = if include_owned_session {
            PtyProcessScope::capture(root)?
        } else {
            PtyProcessScope::capture_lineage(root)?
        };
        #[cfg(target_os = "macos")]
        if let Some(proof) = pty_proof.as_ref() {
            proof.arm_process_capability(scope.live_root())?;
        }
        let closure_identity = pty_proof
            .as_ref()
            .and_then(PtySessionProof::process_closure_identity)
            .unwrap_or_else(crate::process::ProcessClosureIdentity::fresh);
        let events = initialize_events(scope.live_root());
        let boundary = super::super::controller_boundary::ProcessDiscoveryBoundary::capture(
            &scope.live_root(),
            excluded,
        )?;
        let mut tree = Self {
            scope,
            captured: HashMap::new(),
            boundary,
            retiring: HashSet::new(),
            lineage: CapturedLineage::default(),
            events,
            pty_proof,
            pty_scan: PtyCapabilityScanState::default(),
            lineage_settled: false,
            lineage_integrity_lost: false,
            capability_scan_pending: false,
            capability_scheduling_complete: false,
            capability_scan_activity: Default::default(),
            discovery_epoch: discovery::DiscoveryScanEpoch::default(),
            discovery_authoritative_empty: false,
            discovery_publication_audit: Default::default(),
            frozen_fork_reconciliation_required: false,
            exact_containment_lineage: Default::default(),
            exact_lineage_containment: None,
            ownership: SessionOwnership::Tracking,
            root_exit: root_exit::RootExitState::Live,
            destructive: DestructiveSessionAccess::capture(),
            pty_master_released: false,
            explicit_bootstrap_failure: false,
            #[cfg(any(test, feature = "adversarial-test-hooks"))]
            injected_refresh_failure: None,
            #[cfg(target_os = "macos")]
            pty_local_alias_release_prepared: false,
            closure_version: super::ProcessClosureVersion::initial(closure_identity),
            #[cfg(any(test, feature = "runtime-component-tests"))]
            test_state: Default::default(),
        };
        let initial = tree.refresh()?;
        ensure!(
            initial.contains(&root),
            compact::INITIAL_ROOT_MISSING_DETAIL
        );
        Ok(tree)
    }
}
