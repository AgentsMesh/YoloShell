use super::super::macos_liveness::MacosGlobalScanMode;
use crate::process::inventory_snapshot::InventoryEntry;
use crate::process::inventory_stable::StableInventory;
use crate::process::{LiveProcessIdentity, ProcessIdentity};
use anyhow::Result;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;
#[path = "pty_session_released_ancestry_macos.rs"]
mod ancestry;
#[path = "../pty_session_transferable_universe_macos.rs"]
mod transferable;
use ancestry::ExternalAncestry;
pub(super) use transferable::transferable_candidate_universe;

const CANDIDATE_ORDER_DETAIL: &str = "candidate ordering must retain an unresolved lineage";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct ReleasedCandidate {
    pub(super) live: LiveProcessIdentity,
    pub(super) durable: ProcessIdentity,
    pub(super) parent_lineage: u64,
    pub(super) stable: bool,
    pub(super) capability_root: bool,
    pub(super) environment_eligible: bool,
}

pub(super) fn candidate_universe(
    inventory: &StableInventory,
    minimum_start_time: u64,
    root_lineage: u64,
) -> Result<Arc<[ReleasedCandidate]>> {
    let entries = inventory
        .observed
        .iter()
        .map(|entry| (entry.lineage_id(), *entry))
        .collect::<HashMap<_, _>>();
    let mut external = ExternalAncestry::new(minimum_start_time, root_lineage);
    let mut candidates = HashMap::new();
    for entry in inventory.observed.iter().copied() {
        if !could_inherit(
            entry,
            &entries,
            &mut external,
            minimum_start_time,
            root_lineage,
        ) {
            continue;
        }
        let live = entry.live_identity()?;
        candidates.insert(
            live.lineage_id(),
            ReleasedCandidate {
                live,
                durable: entry.identity(),
                parent_lineage: entry.parent_lineage_id(),
                stable: inventory.stable_lineages.contains(&live.lineage_id()),
                capability_root: entry.lineage_id() == root_lineage,
                environment_eligible: true,
            },
        );
    }
    Ok(order_candidates(candidates).into())
}

pub(super) fn scan_candidate_universe(
    inventory: &StableInventory,
    mode: MacosGlobalScanMode,
    minimum_start_time: u64,
    root_lineage: u64,
) -> Result<Arc<[ReleasedCandidate]>> {
    match mode {
        MacosGlobalScanMode::LineageEnvironment
        | MacosGlobalScanMode::PrematureBootstrapRelease => {
            candidate_universe(inventory, minimum_start_time, root_lineage)
        }
        MacosGlobalScanMode::CustodyPtyObject | MacosGlobalScanMode::RecoveryCustodyPtyObject => {
            transferable_candidate_universe(inventory, minimum_start_time, root_lineage)
        }
        MacosGlobalScanMode::TransferableWriterPresent => {
            transferable_candidate_universe(inventory, minimum_start_time, root_lineage)
        }
        MacosGlobalScanMode::TransferableWriterReleased => {
            transferable_candidate_universe(inventory, minimum_start_time, root_lineage)
        }
        MacosGlobalScanMode::ReleasedDiscoveryRetained => Ok(Arc::from([])),
    }
}

pub(super) fn exclude_candidates(
    current: Arc<[ReleasedCandidate]>,
    excluded: &HashSet<LiveProcessIdentity>,
) -> Arc<[ReleasedCandidate]> {
    if excluded.is_empty() {
        return current;
    }
    current
        .iter()
        .filter(|candidate| !excluded.contains(&candidate.live))
        .copied()
        .collect::<Vec<_>>()
        .into()
}

fn order_candidates(candidates: HashMap<u64, ReleasedCandidate>) -> Vec<ReleasedCandidate> {
    let mut children = HashMap::<u64, Vec<u64>>::new();
    let mut blocked = HashSet::new();
    let mut ready = BinaryHeap::new();
    for candidate in candidates.values().copied() {
        let lineage = candidate.live.lineage_id();
        if candidate.parent_lineage != lineage && candidates.contains_key(&candidate.parent_lineage)
        {
            blocked.insert(lineage);
            children
                .entry(candidate.parent_lineage)
                .or_default()
                .push(lineage);
        } else {
            ready.push(Reverse(candidate_key(candidate)));
        }
    }
    let mut ordered = Vec::with_capacity(candidates.len());
    let mut emitted = HashSet::new();
    while emitted.len() < candidates.len() {
        let lineage = next_lineage(&candidates, &blocked, &emitted, &mut ready);
        if !emitted.insert(lineage) {
            continue;
        }
        let candidate = candidates[&lineage];
        ordered.push(candidate);
        for child in children.remove(&lineage).unwrap_or_default() {
            if blocked.remove(&child) {
                ready.push(Reverse(candidate_key(candidates[&child])));
            }
        }
    }
    ordered
}

fn next_lineage(
    candidates: &HashMap<u64, ReleasedCandidate>,
    blocked: &HashSet<u64>,
    emitted: &HashSet<u64>,
    ready: &mut BinaryHeap<Reverse<(u32, u64)>>,
) -> u64 {
    while let Some(Reverse((_pid, lineage))) = ready.pop() {
        if !emitted.contains(&lineage) {
            return lineage;
        }
    }
    candidates
        .values()
        .filter(|candidate| {
            let lineage = candidate.live.lineage_id();
            blocked.contains(&lineage) && !emitted.contains(&lineage)
        })
        .min_by_key(|candidate| candidate_key(**candidate))
        .expect(CANDIDATE_ORDER_DETAIL)
        .live
        .lineage_id()
}

fn candidate_key(candidate: ReleasedCandidate) -> (u32, u64) {
    (candidate.live.pid(), candidate.live.lineage_id())
}

fn could_inherit(
    entry: InventoryEntry,
    entries: &HashMap<u64, InventoryEntry>,
    external: &mut ExternalAncestry,
    minimum_start_time: u64,
    root_lineage: u64,
) -> bool {
    let mut current = entry;
    let mut visited = HashSet::new();
    loop {
        let lineage = current.lineage_id();
        if lineage == root_lineage {
            return true;
        }
        if current.start_time < minimum_start_time {
            return false;
        }
        if !visited.insert(lineage) {
            return true;
        }
        let parent = current.parent_lineage_id();
        let Some(parent) = entries.get(&parent).copied() else {
            return external.could_inherit(current.parent_pid, parent);
        };
        current = parent;
    }
}
