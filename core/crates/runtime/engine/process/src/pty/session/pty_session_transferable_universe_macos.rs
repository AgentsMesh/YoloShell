use super::{ReleasedCandidate, order_candidates};
use crate::process::inventory_stable::StableInventory;
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(in crate::process::pty_session_proof) fn transferable_candidate_universe(
    inventory: &StableInventory,
    minimum_start_time: u64,
    root_lineage: u64,
) -> Result<Arc<[ReleasedCandidate]>> {
    let environment_lineages =
        environment_evidence_lineages(inventory, minimum_start_time, root_lineage);
    let candidates = inventory
        .observed
        .iter()
        .copied()
        .map(|entry| {
            let live = entry.live_identity()?;
            Ok((
                live.lineage_id(),
                ReleasedCandidate {
                    live,
                    durable: entry.identity(),
                    parent_lineage: entry.parent_lineage_id(),
                    stable: inventory.stable_lineages.contains(&live.lineage_id()),
                    capability_root: entry.lineage_id() == root_lineage,
                    environment_eligible: environment_lineages.contains(&live.lineage_id()),
                },
            ))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    Ok(order_candidates(candidates).into())
}

fn environment_evidence_lineages(
    inventory: &StableInventory,
    minimum_start_time: u64,
    root_lineage: u64,
) -> HashSet<u64> {
    let entries = inventory
        .entries
        .iter()
        .map(|entry| (entry.lineage_id(), *entry))
        .collect::<HashMap<_, _>>();
    entries
        .keys()
        .copied()
        .filter(|lineage| {
            environment_evidence_reaches_root(*lineage, &entries, minimum_start_time, root_lineage)
        })
        .collect()
}

fn environment_evidence_reaches_root(
    mut lineage: u64,
    entries: &HashMap<u64, crate::process::inventory_snapshot::InventoryEntry>,
    minimum_start_time: u64,
    root_lineage: u64,
) -> bool {
    let mut visited = HashSet::new();
    loop {
        if lineage == root_lineage {
            return true;
        }
        if !visited.insert(lineage) {
            return false;
        }
        let Some(entry) = entries.get(&lineage) else {
            return false;
        };
        if entry.start_time < minimum_start_time {
            return false;
        }
        lineage = entry.parent_lineage_id();
    }
}
