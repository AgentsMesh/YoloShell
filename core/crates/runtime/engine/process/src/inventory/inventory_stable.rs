use super::captured_lineage::ParentCandidates;
use super::constants::{
    MAX_PROCESS_INVENTORY_IDS, NO_PARENT_LINEAGE_ID, PROCESS_INVENTORY_CAPTURE_ATTEMPTS,
    PROCESS_INVENTORY_RETRY_INTERVAL,
};
use super::inventory_snapshot::{InventoryEntry, inventory};
use super::{LiveProcessIdentity, ProcessIdentity};
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};

#[cfg(not(target_os = "linux"))]
const STABLE_DUPLICATE_LIVE_DETAIL: &str = "stable inventory contains a duplicate live identity";
const DUPLICATE_PID_DETAIL: &str = "process inventory contains a duplicate pid";
const DUPLICATE_LIVE_DETAIL: &str = "process inventory contains a duplicate live identity";
const SELF_PARENT_LINEAGE_DETAIL: &str = "process inventory contains a self-parent lineage";
pub(super) const INVENTORY_CAPTURE_EXHAUSTED_CONTEXT: &str =
    "capture a complete process inventory after bounded retries";
pub(super) const STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE: usize = 2;
pub(super) const MAX_STABLE_INVENTORY_OBSERVATIONS: usize =
    MAX_PROCESS_INVENTORY_IDS * STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
const _: () = assert!(PROCESS_INVENTORY_CAPTURE_ATTEMPTS > 0);

pub(super) struct StableInventory {
    pub(super) entries: Vec<InventoryEntry>,
    pub(super) observed: Vec<InventoryEntry>,
    pub(super) by_pid: HashMap<u32, InventoryEntry>,
    #[cfg(not(target_os = "linux"))]
    pub(super) by_live: HashMap<LiveProcessIdentity, InventoryEntry>,
    pub(super) trusted_parents: ParentCandidates,
    pub(super) possible_parents: ParentCandidates,
    pub(super) observed_sessions: HashMap<u64, HashSet<u32>>,
    pub(super) observed_lineages: HashSet<u64>,
    pub(super) stable_lineages: HashSet<u64>,
    pub(super) unstable_lineages: HashSet<u64>,
}

impl StableInventory {
    pub(super) fn capture() -> Result<Self> {
        let snapshots = std::iter::repeat_with(capture_snapshot)
            .take(STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE)
            .collect::<Result<Vec<_>>>()?;
        let [first, second]: [Vec<InventoryEntry>; STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE] =
            snapshots.try_into().unwrap_or_else(|_| unreachable!());
        let second_by_pid = index_by_pid(&second)?;
        let entries = first
            .iter()
            .copied()
            .filter(|entry| {
                second_by_pid
                    .get(&entry.pid)
                    .is_some_and(|second| entry.stable_with(*second))
            })
            .collect::<Vec<_>>();
        let observed = first.iter().chain(&second).copied().collect();
        Self::from_entries(entries, observed)
    }

    fn from_entries(entries: Vec<InventoryEntry>, observed: Vec<InventoryEntry>) -> Result<Self> {
        let by_pid = index_by_pid(&entries)?;
        #[cfg(not(target_os = "linux"))]
        let mut by_live = HashMap::new();
        let mut stable_lineages = HashSet::new();
        for entry in entries.iter().copied() {
            #[cfg(not(target_os = "linux"))]
            let live = entry.live_identity()?;
            #[cfg(not(target_os = "linux"))]
            ensure!(
                by_live.insert(live, entry).is_none(),
                STABLE_DUPLICATE_LIVE_DETAIL
            );
            stable_lineages.insert(entry.lineage_id());
        }
        let mut possible_parents = ParentCandidates::new();
        let mut observed_sessions: HashMap<u64, HashSet<u32>> = HashMap::new();
        let mut observed_lineages = HashSet::new();
        for entry in observed.iter().copied() {
            let lineage = entry.lineage_id();
            observed_lineages.insert(lineage);
            observed_sessions
                .entry(lineage)
                .or_default()
                .insert(entry.session_id);
            if let Some(edge) = entry.lineage_edge() {
                possible_parents
                    .entry(edge.child)
                    .or_default()
                    .insert(edge.parent);
            }
        }
        let trusted_parents =
            super::inventory_evidence::trusted_parents(&possible_parents, &stable_lineages);
        let unstable_lineages = observed_lineages
            .difference(&stable_lineages)
            .copied()
            .collect();
        Ok(Self {
            entries,
            observed,
            by_pid,
            #[cfg(not(target_os = "linux"))]
            by_live,
            trusted_parents,
            possible_parents,
            observed_sessions,
            observed_lineages,
            stable_lineages,
            unstable_lineages,
        })
    }

    pub(super) fn entries(&self) -> &[InventoryEntry] {
        &self.entries
    }

    pub(super) fn root_entry(&self, identity: ProcessIdentity) -> Option<InventoryEntry> {
        self.by_pid
            .get(&identity.pid)
            .copied()
            .filter(|entry| entry.identity() == identity)
    }

    pub(super) fn contains(&self, identity: &LiveProcessIdentity) -> bool {
        #[cfg(target_os = "linux")]
        return self
            .by_pid
            .get(&identity.pid())
            .is_some_and(|entry| entry.identity() == identity.durable_projection());
        #[cfg(not(target_os = "linux"))]
        self.by_live.contains_key(&identity)
    }

    pub(super) fn session_for(&self, identity: &LiveProcessIdentity) -> Option<u32> {
        #[cfg(target_os = "linux")]
        return self
            .by_pid
            .get(&identity.pid())
            .filter(|entry| entry.identity() == identity.durable_projection())
            .map(|entry| entry.session_id);
        #[cfg(not(target_os = "linux"))]
        self.by_live.get(&identity).map(|entry| entry.session_id)
    }

    pub(super) fn live_identity(
        &self,
        entry: InventoryEntry,
    ) -> Result<Option<LiveProcessIdentity>> {
        #[cfg(target_os = "linux")]
        return entry.live_identity();
        #[cfg(not(target_os = "linux"))]
        return entry.live_identity().map(Some);
    }
}

fn capture_snapshot() -> Result<Vec<InventoryEntry>> {
    let mut attempts = std::iter::repeat_n((), PROCESS_INVENTORY_CAPTURE_ATTEMPTS).peekable();
    while attempts.next().is_some() {
        match inventory() {
            Ok(snapshot) => {
                validate_snapshot(&snapshot)?;
                return Ok(snapshot);
            }
            Err(error) if attempts.peek().is_none() => {
                return Err(error).context(INVENTORY_CAPTURE_EXHAUSTED_CONTEXT);
            }
            Err(_) => std::thread::sleep(PROCESS_INVENTORY_RETRY_INTERVAL),
        }
    }
    unreachable!()
}

fn validate_snapshot(entries: &[InventoryEntry]) -> Result<()> {
    let mut pids = HashSet::new();
    let mut lineage = HashSet::new();
    for entry in entries.iter().copied() {
        entry.validate_identity()?;
        ensure!(pids.insert(entry.pid), DUPLICATE_PID_DETAIL);
        ensure!(lineage.insert(entry.lineage_id()), DUPLICATE_LIVE_DETAIL);
        ensure!(
            entry.parent_lineage_id() == NO_PARENT_LINEAGE_ID
                || entry.parent_lineage_id() != entry.lineage_id(),
            SELF_PARENT_LINEAGE_DETAIL
        );
    }
    Ok(())
}

fn index_by_pid(entries: &[InventoryEntry]) -> Result<HashMap<u32, InventoryEntry>> {
    let mut indexed = HashMap::new();
    for entry in entries.iter().copied() {
        ensure!(
            indexed.insert(entry.pid, entry).is_none(),
            DUPLICATE_PID_DETAIL
        );
    }
    Ok(indexed)
}
