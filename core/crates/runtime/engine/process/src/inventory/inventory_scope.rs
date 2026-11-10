use super::captured_lineage::CapturedLineage;
use super::inventory::{ProcessGroupIdentity, PtyProcessScope, SessionOwnershipProof};
use super::inventory_stable::StableInventory;
use super::{LiveProcessIdentity, ProcessIdentity};
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;

const ROOT_MISSING_DETAIL: &str = "PTY root identity is no longer running";
const ROOT_INVENTORY_MISSING_DETAIL: &str = "PTY root is absent from stable process inventory";
const ROOT_INVENTORY_CHANGED_DETAIL: &str = "PTY root identity changed during process inventory";
const FOREGROUND_GROUP_INVALID_DETAIL: &str = "invalid foreground process group";
const ROOT_CHANGED_BEFORE_SIGNAL_DETAIL: &str = "PTY root identity changed before signal";
const FOREGROUND_GROUP_OUTSIDE_SCOPE_DETAIL: &str =
    "foreground process group is outside the PTY scope";
const FOREGROUND_IDENTITY_CHANGED_DETAIL: &str =
    "foreground process identity changed during inventory";

impl PtyProcessScope {
    pub fn capture(root: ProcessIdentity) -> Result<Self> {
        Self::capture_with_session(root, true)
    }

    pub(crate) fn capture_lineage(root: ProcessIdentity) -> Result<Self> {
        Self::capture_with_session(root, false)
    }

    fn capture_with_session(root: ProcessIdentity, include_owned_session: bool) -> Result<Self> {
        let expected_live = LiveProcessIdentity::capture(root)?.context(ROOT_MISSING_DETAIL)?;
        let inventory = StableInventory::capture()?;
        let root_entry = inventory
            .root_entry(root)
            .context(ROOT_INVENTORY_MISSING_DETAIL)?;
        ensure!(
            inventory.contains(&expected_live) && expected_live.try_matches()?,
            ROOT_INVENTORY_CHANGED_DETAIL
        );
        Ok(Self {
            root: expected_live,
            durable_root: root,
            owned_session_id: (include_owned_session && root_entry.session_id == root.pid)
                .then_some(root.pid),
        })
    }

    pub(crate) fn root(&self) -> ProcessIdentity {
        self.durable_root
    }

    pub(super) fn live_root(&self) -> LiveProcessIdentity {
        self.root.clone()
    }

    #[cfg(any(test, feature = "runtime-component-tests"))]
    pub fn identities(&self, anchors: &HashSet<ProcessIdentity>) -> Result<Vec<ProcessIdentity>> {
        let pty_scan = super::pty_session_proof::PtyCapabilityScanState::default();
        let live = anchors
            .iter()
            .copied()
            .map(LiveProcessIdentity::capture)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        let boundary =
            super::controller_boundary::ProcessDiscoveryBoundary::capture(&self.root, Vec::new())?;
        Ok(self
            .observe_with_proof(
                &live,
                &boundary,
                &CapturedLineage::default(),
                SessionOwnershipProof::LiveWitness,
                None,
                &pty_scan,
                false,
                super::inventory_observation::CapabilityObservationMode::Scan,
            )?
            .identities
            .into_iter()
            .map(|process| process.durable)
            .collect())
    }

    /// Read-only membership observation. The retained signal capability never
    /// leaves the process owner; callers receive no permission to signal.
    pub fn foreground_contains(&self, group_id: i32, expected: ProcessIdentity) -> Result<bool> {
        self.foreground_group(group_id)?.contains_current(expected)
    }

    pub fn foreground_group(&self, group_id: i32) -> Result<ProcessGroupIdentity> {
        ensure!(
            group_id.is_positive(),
            "{FOREGROUND_GROUP_INVALID_DETAIL}: {group_id}"
        );
        let group_id = group_id as u32;
        let inventory = StableInventory::capture()?;
        ensure!(
            inventory.contains(&self.root) && self.root.try_matches()?,
            ROOT_CHANGED_BEFORE_SIGNAL_DETAIL
        );
        let lineage_roots = HashSet::from([self.root.lineage_id()]);
        let live_session = self.live_session_id(
            &inventory,
            std::slice::from_ref(&self.root),
            SessionOwnershipProof::LiveWitness,
        );
        let lineage = CapturedLineage::default();
        let stable_session = inventory.stable_session_lineages(live_session);
        let seeds = lineage_roots.union(&stable_session).copied().collect();
        let lineage_descendants = inventory.trusted_descendants(&seeds, &lineage);
        let owned = seeds
            .union(&lineage_descendants)
            .copied()
            .collect::<HashSet<_>>();
        let mut members = Vec::new();
        for entry in inventory.entries().iter().copied() {
            if entry.process_group == group_id && owned.contains(&entry.lineage_id()) {
                if let Some(identity) = inventory.live_identity(entry)? {
                    members.push(identity);
                }
            }
        }
        ensure!(!members.is_empty(), FOREGROUND_GROUP_OUTSIDE_SCOPE_DETAIL);
        let members_stable = members
            .iter()
            .map(LiveProcessIdentity::try_matches)
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .all(|matches| matches);
        ensure!(
            inventory.contains(&self.root) && self.root.try_matches()? && members_stable,
            FOREGROUND_IDENTITY_CHANGED_DETAIL
        );
        Ok(ProcessGroupIdentity::verified(group_id, members))
    }

    pub(super) fn live_session_id(
        &self,
        inventory: &StableInventory,
        live_roots: &[LiveProcessIdentity],
        proof: SessionOwnershipProof,
    ) -> Option<u32> {
        let session_id = self.owned_session_id?;
        match proof {
            SessionOwnershipProof::RootExitedUnreaped => Some(session_id),
            SessionOwnershipProof::RootGoneWithoutWait
            | SessionOwnershipProof::SessionScanDisabled
            | SessionOwnershipProof::SessionRetired => None,
            SessionOwnershipProof::LiveWitness => live_roots
                .iter()
                .any(|identity| inventory.session_for(identity) == Some(session_id))
                .then_some(session_id),
        }
    }
}
