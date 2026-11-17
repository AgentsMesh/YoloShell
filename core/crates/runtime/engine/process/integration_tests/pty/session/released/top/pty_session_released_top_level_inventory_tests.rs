use crate::process::inventory_snapshot::InventoryEntry;
use crate::process::inventory_stable::StableInventory;
use std::collections::{HashMap, HashSet};

pub(super) fn stable_inventory(process: InventoryEntry) -> StableInventory {
    let lineage = process.lineage_id();
    StableInventory {
        entries: vec![process],
        observed: vec![process],
        by_pid: HashMap::from([(process.pid, process)]),
        by_live: HashMap::from([(process.live_identity().unwrap(), process)]),
        trusted_parents: HashMap::new(),
        possible_parents: HashMap::new(),
        observed_sessions: HashMap::new(),
        observed_lineages: HashSet::from([lineage]),
        stable_lineages: HashSet::from([lineage]),
        unstable_lineages: HashSet::new(),
    }
}

pub(super) fn empty_inventory() -> StableInventory {
    StableInventory {
        entries: Vec::new(),
        observed: Vec::new(),
        by_pid: HashMap::new(),
        by_live: HashMap::new(),
        trusted_parents: HashMap::new(),
        possible_parents: HashMap::new(),
        observed_sessions: HashMap::new(),
        observed_lineages: HashSet::new(),
        stable_lineages: HashSet::new(),
        unstable_lineages: HashSet::new(),
    }
}
