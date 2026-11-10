use crate::process::ProcessIdentity;
use crate::process::captured_lineage::ParentCandidates;
use crate::process::inventory_snapshot::InventoryEntry;
use crate::process::inventory_stable::StableInventory;
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};

const GUARDIAN_INVENTORY_MISSING_DETAIL: &str =
    "Guardian is absent from stable PTY holder inventory";
const GUARDIAN_GENERATION_CHANGED_DETAIL: &str =
    "Guardian identity changed during PTY holder inventory";
const HOLDER_ANCESTRY_AMBIGUOUS_DETAIL: &str =
    "PTY holder cannot be proven independent from Guardian ancestry";
const GUARDIAN_LINEAGE_CYCLE_DETAIL: &str = "Guardian ancestor inventory contains a cycle";

pub(super) struct GuardianAncestry {
    guardian: ProcessIdentity,
    guardian_lineage: u64,
    protected: HashSet<u64>,
    complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum HolderDisposition {
    Contain,
    Exclude,
}

pub(super) fn guardian_ancestry(
    inventory: &StableInventory,
    guardian: ProcessIdentity,
) -> Result<GuardianAncestry> {
    let guardian_lineage = inventory
        .entries()
        .iter()
        .copied()
        .find(|entry| entry.identity() == guardian)
        .map(|entry| entry.lineage_id())
        .context(GUARDIAN_INVENTORY_MISSING_DETAIL)?;
    ensure!(guardian.matches(), GUARDIAN_GENERATION_CHANGED_DETAIL);
    let observed = observed_parents(inventory);
    let (protected, complete) = proven_ancestor_prefix(
        guardian_lineage,
        &observed,
        &inventory.possible_parents,
        &inventory.trusted_parents,
    );
    ensure!(guardian.matches(), GUARDIAN_GENERATION_CHANGED_DETAIL);
    Ok(GuardianAncestry {
        guardian,
        guardian_lineage,
        protected,
        complete,
    })
}

fn observed_parents(inventory: &StableInventory) -> ParentCandidates {
    let mut parents = HashMap::new();
    for entry in inventory.observed.iter().copied() {
        parents
            .entry(entry.lineage_id())
            .or_insert_with(HashSet::new)
            .insert(entry.parent_lineage_id());
    }
    parents
}

fn proven_ancestor_prefix(
    guardian: u64,
    observed: &ParentCandidates,
    possible: &ParentCandidates,
    trusted: &ParentCandidates,
) -> (HashSet<u64>, bool) {
    let root = super::super::super::constants::NO_PARENT_LINEAGE_ID;
    let mut protected = HashSet::new();
    let mut lineage = guardian;
    loop {
        if !protected.insert(lineage) {
            return (protected, false);
        }
        let Some(parents) = observed.get(&lineage) else {
            return (protected, false);
        };
        if parents.len() == 1 && parents.contains(&root) {
            return (protected, true);
        }
        if parents.len() != 1
            || parents.contains(&root)
            || possible.get(&lineage) != Some(parents)
            || trusted.get(&lineage) != Some(parents)
        {
            return (protected, false);
        }
        lineage = *parents.iter().next().expect(GUARDIAN_LINEAGE_CYCLE_DETAIL);
    }
}

impl GuardianAncestry {
    pub(super) fn classify(&self, entry: InventoryEntry) -> Result<HolderDisposition> {
        ensure!(self.guardian.matches(), GUARDIAN_GENERATION_CHANGED_DETAIL);
        if entry.identity() == self.guardian || self.protected.contains(&entry.lineage_id()) {
            return Ok(HolderDisposition::Exclude);
        }
        if self.complete || entry.start_time > self.guardian.start_time {
            return Ok(HolderDisposition::Contain);
        }
        anyhow::bail!(
            "{HOLDER_ANCESTRY_AMBIGUOUS_DETAIL}: pid={}, lineage={}, guardian_lineage={}",
            entry.pid,
            entry.lineage_id(),
            self.guardian_lineage,
        )
    }

    #[cfg(test)]
    pub(super) fn incomplete_for_test(
        guardian: ProcessIdentity,
        guardian_lineage: u64,
        protected: HashSet<u64>,
    ) -> Self {
        Self {
            guardian,
            guardian_lineage,
            protected,
            complete: false,
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../../integration_tests/pty/session/passive/pty_session_passive_master_holder_lineage_macos_tests.rs"]
mod tests;
