use super::*;
use crate::process::PROCESS_DISCOVERY_CAPABILITY_ENV;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_discovery_test_support::DiscoveryTreeTestFixture;
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_stable::STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};

#[path = "captured_tree_post_inventory_fork_test_support.rs"]
mod support;
use support::{assert_covered_uncertainty, assert_pending_uncertainty, refresh_until_covered};

const TEST_CAPABILITY: &str = "post-inventory-fork-test";

#[test]
fn queued_coalesced_fork_remains_uncertain_after_a_new_child_is_registered() {
    let mut fixture = DiscoveryTreeTestFixture::spawn(TEST_CAPABILITY);
    let child = fixture.spawn_holder();
    let _escaped = fixture.spawn_holder();
    fixture.release_local_writer();
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
    let root_only = vec![root_entry];
    let with_child = vec![root_entry, child_entry];
    fixture.tree.begin_discovery_cleanup_internal();
    let _hook = fixture
        .tree
        .queue_fork_after_next_inventory(fixture.tree.live_root());
    refresh(&mut fixture.tree, root_only);
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Pending);

    {
        let _inventory = install_test_inventory(std::iter::repeat_n(
            with_child.clone(),
            STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * EVENT_RECONCILIATION_LIMIT,
        ));
        assert!(fixture.tree.refresh_if_changed().unwrap());
    }
    refresh_until_covered(&mut fixture.tree, with_child);
    assert_covered_uncertainty(&fixture.tree);
}

#[test]
fn queued_fork_cannot_borrow_a_previously_registered_child_edge() {
    let root = TestProcess::spawn();
    let child =
        TestProcess::spawn_with_environment(PROCESS_DISCOVERY_CAPABILITY_ENV, TEST_CAPABILITY);
    let mut tree = proof_tree(&root);
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let child_entry = entry(child.identity, root.identity.pid, child.identity.pid);
    let entries = vec![root_entry, child_entry];
    refresh(&mut tree, entries.clone());
    assert!(tree.events.requires_reconciliation());
    tree.begin_discovery_cleanup_internal();
    let _hook = tree.queue_fork_after_next_inventory(tree.live_root());
    refresh(&mut tree, entries.clone());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);

    {
        let _inventory = install_test_inventory(std::iter::repeat_n(
            entries.clone(),
            STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * EVENT_RECONCILIATION_LIMIT,
        ));
        assert!(tree.refresh_if_changed().unwrap());
    }
    let snapshot = tree.state_snapshot_for_test();

    assert!(snapshot.capability_scheduling_complete);
    assert!(!snapshot.capability_coverage_complete);
    assert!(tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert_pending_uncertainty(&tree);
}

#[test]
fn queued_fork_without_an_exact_edge_remains_uncertain() {
    let root = TestProcess::spawn();
    let mut tree = proof_tree(&root);
    let entries = vec![entry(root.identity, NO_PARENT_PID, root.identity.pid)];
    tree.begin_discovery_cleanup_internal();
    let _hook = tree.queue_fork_after_next_inventory(tree.live_root());
    refresh(&mut tree, entries.clone());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);

    {
        let _inventory = install_test_inventory(std::iter::repeat_n(
            entries.clone(),
            STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * EVENT_RECONCILIATION_LIMIT,
        ));
        assert!(tree.refresh_if_changed().unwrap());
    }
    let snapshot = tree.state_snapshot_for_test();

    assert!(snapshot.capability_scheduling_complete);
    assert!(!snapshot.capability_coverage_complete);
    assert!(tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert_pending_uncertainty(&tree);
}

#[test]
fn post_exit_fork_without_a_live_edge_starts_a_fresh_authoritative_epoch() {
    let mut fixture = DiscoveryTreeTestFixture::spawn(TEST_CAPABILITY);
    fixture.exit_root();
    fixture.wait_for_kernel_eof();
    refresh(&mut fixture.tree, Vec::new());
    refresh(&mut fixture.tree, Vec::new());
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Complete);
    let _hook = fixture
        .tree
        .queue_fork_after_next_inventory(fixture.tree.live_root());
    refresh(&mut fixture.tree, Vec::new());

    assert!(fixture.tree.events.requires_reconciliation());
    fixture.tree.restart_discovery_after_event(true);
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    refresh(&mut fixture.tree, Vec::new());

    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Complete);
    assert!(!fixture.tree.lineage_integrity_lost);
    assert!(!fixture.tree.events.has_unresolved_fork());
}

#[test]
fn post_exit_coalesced_fork_without_a_proof_cannot_clear_uncertainty() {
    let root = TestProcess::spawn();
    let child = TestProcess::spawn();
    let _escaped = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let child_entry = entry(child.identity, root.identity.pid, root.identity.pid);
    let root_only = vec![root_entry];
    let with_child = vec![root_entry, child_entry];
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    let _hook = tree.queue_fork_after_next_inventory(tree.live_root());
    refresh(&mut tree, root_only);

    let _inventory = install_test_inventory(std::iter::repeat_n(
        with_child,
        STABLE_INVENTORY_SNAPSHOTS_PER_CAPTURE * EVENT_RECONCILIATION_LIMIT,
    ));
    assert!(tree.refresh_if_changed().unwrap());

    assert!(tree.lineage_integrity_lost);
    assert!(tree.events.has_unresolved_fork());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Inactive);
}

fn proof_tree(root: &TestProcess) -> CapturedProcessTree {
    CapturedProcessTree::capture_with_discovery_internal(root.identity, TEST_CAPABILITY.to_owned())
        .unwrap()
}

fn refresh(
    tree: &mut CapturedProcessTree,
    entries: Vec<crate::process::inventory_snapshot::InventoryEntry>,
) {
    let _inventory = install_test_inventory([entries.clone(), entries]);
    tree.refresh().unwrap();
}
