use super::MacosPassivePtyMaster;
use crate::process::ProcessIdentity;
use crate::process::inventory_snapshot::InventoryEntry;
use crate::process::inventory_stable::StableInventory;
use crate::pty_custody::CustodyRuntimeIdentity;
use anyhow::{Context, Result, ensure};
use std::collections::HashSet;
use std::path::Path;

const HOLDER_GENERATION_CHANGED_DETAIL: &str =
    "positive PTY holder identity changed before exact containment";
const HOLDER_INVENTORY_UNSTABLE_DETAIL: &str =
    "positive PTY holder is absent from the stable exact inventory";
const HOLDER_OBJECT_MISMATCH_DETAIL: &str = "positive PTY holder lacks the exact PTY descriptor";
const HOLDER_BEFORE_QUERY_CONTEXT: &str = "capture initial positive PTY holder universe";
const HOLDER_AFTER_QUERY_CONTEXT: &str = "confirm final positive PTY holder universe";
const HOLDER_UNIVERSE_CHANGED_DETAIL: &str =
    "positive PTY holder universe changed during exact discovery";

#[derive(Clone, Copy)]
pub(in crate::process) struct MacosPtyHolderExclusions {
    worker: ProcessIdentity,
    child: ProcessIdentity,
}

impl MacosPtyHolderExclusions {
    pub(in crate::process) fn from_runtime(runtime: &CustodyRuntimeIdentity) -> Self {
        Self {
            worker: ProcessIdentity {
                pid: runtime.worker_pid,
                start_time: runtime.worker_start_time,
            },
            child: ProcessIdentity {
                pid: runtime.child_pid,
                start_time: runtime.child_start_time,
            },
        }
    }

    fn contains(self, identity: ProcessIdentity) -> bool {
        identity == self.worker || identity == self.child
    }
}

impl MacosPassivePtyMaster {
    pub(in crate::process) fn exact_holder_identities_at(
        &self,
        hint_path: &Path,
        exclusions: &MacosPtyHolderExclusions,
    ) -> Result<Vec<ProcessIdentity>> {
        self.revalidate_self()?;
        let before = super::super::macos_path_holders::path_holders(
            hint_path,
            super::super::super::constants::MAX_PTY_PROOF_CANDIDATES,
        )
        .context(HOLDER_BEFORE_QUERY_CONTEXT)?;
        let inventory = StableInventory::capture()?;
        let guardian = ProcessIdentity::capture(std::process::id())?;
        let ancestry = super::holder_lineage::guardian_ancestry(&inventory, guardian)?;
        let candidates =
            classify_holder_candidates(&before, inventory.entries(), &ancestry, exclusions)?;
        let mut budget =
            super::super::macos_contract::CapabilityScanBudget::start(candidates.len())?;
        let mut holders = Vec::new();
        for entry in candidates {
            let identity = entry.identity();
            let live = inventory
                .live_identity(entry)?
                .ok_or_else(|| anyhow::anyhow!(HOLDER_INVENTORY_UNSTABLE_DETAIL))?;
            ensure!(live.try_matches()?, HOLDER_GENERATION_CHANGED_DETAIL);
            let matched = super::super::macos_descriptors::has_stable_tty_descriptor(
                identity.pid,
                self.identity,
                &mut budget,
            )?;
            ensure!(matched, HOLDER_OBJECT_MISMATCH_DETAIL);
            ensure!(live.try_matches()?, HOLDER_GENERATION_CHANGED_DETAIL);
            holders.push(identity);
        }
        let after = super::super::macos_path_holders::path_holders(
            hint_path,
            super::super::super::constants::MAX_PTY_PROOF_CANDIDATES,
        )
        .context(HOLDER_AFTER_QUERY_CONTEXT)?;
        verify_holder_universe(&before, &after)?;
        self.revalidate_self()?;
        holders.sort_unstable_by_key(|identity| (identity.pid, identity.start_time));
        holders.dedup();
        Ok(holders)
    }
}

fn classify_holder_candidates(
    before: &HashSet<u32>,
    entries: &[InventoryEntry],
    ancestry: &super::holder_lineage::GuardianAncestry,
    exclusions: &MacosPtyHolderExclusions,
) -> Result<Vec<InventoryEntry>> {
    let mut accounted = HashSet::new();
    let mut candidates = Vec::new();
    for entry in entries.iter().copied() {
        if !before.contains(&entry.pid) {
            continue;
        }
        accounted.insert(entry.pid);
        if exclusions.contains(entry.identity()) {
            continue;
        }
        if matches!(
            ancestry.classify(entry)?,
            super::holder_lineage::HolderDisposition::Contain
        ) {
            candidates.push(entry);
        }
    }
    ensure!(
        before.iter().all(|pid| accounted.contains(pid)),
        HOLDER_INVENTORY_UNSTABLE_DETAIL
    );
    Ok(candidates)
}

fn verify_holder_universe(before: &HashSet<u32>, after: &HashSet<u32>) -> Result<()> {
    ensure!(before == after, HOLDER_UNIVERSE_CHANGED_DETAIL);
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/passive/pty_session_passive_master_holder_exclusion_macos_tests.rs"]
mod tests;
