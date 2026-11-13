use super::*;
use crate::process::inventory_observation::cache;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_stable::{STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE, StableInventory};
use crate::process::inventory_test_support::TestProcess;
use std::sync::Arc;

const EVENT_INVENTORY_CAPTURE_COUNT: usize = 2;

#[test]
fn event_invalidation_discards_the_frozen_inventory() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let _inventory = install_test_inventory(std::iter::repeat_n(
        Vec::new(),
        STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * EVENT_INVENTORY_CAPTURE_COUNT,
    ));
    let inventory = Arc::new(StableInventory::capture().unwrap());
    tree.pty_scan
        .install_frozen_inventory_for_test(Arc::clone(&inventory));
    assert!(tree.pty_scan.has_frozen_inventory_for_test());

    tree.restart_discovery_after_event(true);

    assert!(!tree.pty_scan.has_frozen_inventory_for_test());
    let fresh = cache::capture(&tree.pty_scan).unwrap();
    assert!(!Arc::ptr_eq(&inventory, &fresh));
}
