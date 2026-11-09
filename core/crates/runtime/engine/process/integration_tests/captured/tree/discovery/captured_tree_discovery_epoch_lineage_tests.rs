use super::*;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_discovery_test_support::DiscoveryTreeTestFixture;
use crate::process::captured_tree_macos_wait::TEST_TIMEOUT;
use crate::process::inventory_snapshot::{
    install_repeating_test_inventory, install_test_inventory,
};
use crate::process::inventory_test_support::{NO_PARENT_PID, entry};
use std::time::Instant;

const TEST_CAPABILITY: &str = "discovery-epoch-lineage-test";
const CHILD_CAPTURE_TIMEOUT_DETAIL: &str = "released child was not captured before the deadline";
const CLEANUP_TIMEOUT_DETAIL: &str = "discovery cleanup did not complete before the deadline";
const LINEAGE_TIMEOUT_DETAIL: &str = "lineage coverage did not settle before the deadline";
const TICK_IDLE_DETAIL: &str = "discovery convergence stopped requesting production ticks";

#[test]
fn indeterminate_lineage_cannot_consume_a_complete_capability_epoch() {
    let mut fixture = DiscoveryTreeTestFixture::spawn(TEST_CAPABILITY);
    let mut child = fixture.spawn_holder();
    fixture.release_local_writer();
    fixture.tree.begin_discovery_cleanup_internal();
    fixture
        .tree
        .events
        .inject_unresolved_fork(fixture.tree.live_root());
    let root_entry = entry(
        fixture.root.identity,
        NO_PARENT_PID,
        fixture.root.identity.pid,
    );
    let child_entry = entry(
        child.identity,
        fixture.root.identity.pid,
        child.identity.pid,
    );
    let _unstable = install_test_inventory([vec![root_entry, child_entry], vec![root_entry]]);

    fixture.tree.refresh().unwrap();

    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(fixture.tree.lineage_integrity_lost);
    drop(_unstable);
    let complete = vec![root_entry, child_entry];
    let _stable = install_repeating_test_inventory(complete);
    let deadline = Instant::now() + TEST_TIMEOUT;

    while !fixture.tree.lineage_settled
        || fixture.tree.discovery_epoch != DiscoveryScanEpoch::Covered
    {
        run_tracking_tick(&mut fixture.tree, deadline, LINEAGE_TIMEOUT_DETAIL);
    }

    assert!(fixture.tree.capability_scan_pending);
    assert!(fixture.tree.lineage_settled);
    assert!(fixture.tree.events.has_unresolved_fork());
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Covered);
    assert!(fixture.tree.lineage_integrity_lost);
    drop(_stable);

    fixture.exit_root();
    child.terminate();
    fixture.wait_for_kernel_eof();
    let _empty = install_repeating_test_inventory(Vec::new());
    let deadline = Instant::now() + TEST_TIMEOUT;
    while fixture.tree.capability_scan_pending
        || fixture.tree.events.has_unresolved_fork()
        || fixture.tree.discovery_epoch != DiscoveryScanEpoch::Complete
    {
        run_tracking_tick(&mut fixture.tree, deadline, CLEANUP_TIMEOUT_DETAIL);
    }

    assert!(!fixture.tree.capability_scan_pending);
    assert!(!fixture.tree.events.has_unresolved_fork());
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Complete);
}

#[test]
fn unstable_observed_match_survives_its_next_inventory_omission() {
    let mut fixture = DiscoveryTreeTestFixture::spawn(TEST_CAPABILITY);
    let child = fixture.spawn_holder();
    fixture.release_local_writer();
    fixture.tree.begin_discovery_cleanup_internal();
    let root_entry = entry(
        fixture.root.identity,
        NO_PARENT_PID,
        fixture.root.identity.pid,
    );
    let child_entry = entry(
        child.identity,
        fixture.root.identity.pid,
        child.identity.pid,
    );
    let _unstable = install_test_inventory([vec![root_entry, child_entry], vec![root_entry]]);

    fixture.tree.refresh().unwrap();

    assert!(fixture.tree.capability_scan_pending);
    assert!(
        fixture.tree.pty_scan.has_frozen_inventory_for_test(),
        "{:?}",
        fixture.tree.state_snapshot_for_test()
    );
    drop(_unstable);
    let _omitted = install_repeating_test_inventory(vec![root_entry]);
    let deadline = Instant::now() + TEST_TIMEOUT;

    while !fixture.tree.identities().contains(&child.identity) {
        run_tracking_tick(&mut fixture.tree, deadline, CHILD_CAPTURE_TIMEOUT_DETAIL);
    }
    assert!(fixture.tree.refresh_if_changed().unwrap());

    assert!(child.identity.matches());
    assert!(
        fixture.tree.identities().contains(&child.identity),
        "{:?}",
        fixture.tree.state_snapshot_for_test()
    );
    assert!(!fixture.tree.discovery_is_authoritatively_empty_internal());
    assert!(!fixture.tree.discovery_publication_ready_internal().unwrap());
}

fn run_tracking_tick(tree: &mut CapturedProcessTree, deadline: Instant, timeout_detail: &str) {
    let advanced = tree.refresh_if_changed().unwrap();
    let snapshot = tree.state_snapshot_for_test();
    assert!(advanced, "{TICK_IDLE_DETAIL}: {snapshot:#?}");
    assert!(Instant::now() < deadline, "{timeout_detail}: {snapshot:#?}");
    std::thread::yield_now();
}
