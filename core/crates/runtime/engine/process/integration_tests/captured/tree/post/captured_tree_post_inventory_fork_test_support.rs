use super::{CapturedProcessTree, DiscoveryScanEpoch};
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
use crate::process::inventory_snapshot::{InventoryEntry, install_test_inventory};
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use std::time::{Duration, Instant};

const DISCOVERY_RETRY_TIMEOUT: Duration = Duration::from_secs(5);
const DISCOVERY_RETRY_INTERVAL: Duration = Duration::from_millis(1);
const DISCOVERY_RETRY_TIMEOUT_DETAIL: &str = "post-inventory discovery did not converge";

pub(super) fn refresh_until_covered(tree: &mut CapturedProcessTree, entries: Vec<InventoryEntry>) {
    let deadline = Instant::now() + DISCOVERY_RETRY_TIMEOUT;
    while tree.discovery_epoch != DiscoveryScanEpoch::Covered {
        let _inventory = install_test_inventory(std::iter::repeat_n(
            entries.clone(),
            STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * EVENT_RECONCILIATION_LIMIT,
        ));
        assert!(tree.refresh_if_changed().unwrap());
        assert_ne!(tree.discovery_epoch, DiscoveryScanEpoch::Complete);
        assert!(
            Instant::now() < deadline,
            "{DISCOVERY_RETRY_TIMEOUT_DETAIL}: {:?}",
            tree.state_snapshot_for_test()
        );
        std::thread::sleep(DISCOVERY_RETRY_INTERVAL);
    }
}

pub(super) fn assert_covered_uncertainty(tree: &CapturedProcessTree) {
    assert!(tree.lineage_integrity_lost);
    assert!(tree.events.has_unresolved_fork());
    assert!(tree.capability_scan_pending);
    assert!(tree.lineage_settled);
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Covered);
}

pub(super) fn assert_pending_uncertainty(tree: &CapturedProcessTree) {
    let snapshot = tree.state_snapshot_for_test();
    assert!(snapshot.capability_scheduling_complete);
    assert!(!snapshot.capability_coverage_complete);
    assert!(tree.lineage_integrity_lost);
    assert!(tree.events.has_unresolved_fork());
    assert!(tree.capability_scan_pending);
    assert!(tree.lineage_settled);
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
}
