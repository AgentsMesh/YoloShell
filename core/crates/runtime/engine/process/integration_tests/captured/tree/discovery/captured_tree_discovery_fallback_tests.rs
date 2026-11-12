use super::*;
use crate::process::MacosProcessCapabilityIssuer;
use crate::process::captured_tree::discovery::DiscoveryScanEpoch;
use crate::process::captured_tree_discovery_test_support::DiscoveryTreeTestFixture;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::macos_process_capability::test_support;

const TEST_CAPABILITY: &str = "dense-poll-discovery-test";

#[test]
fn proof_backed_stable_empty_dense_poll_converges_without_a_gate() {
    let mut fixture = released_discovery_tree();
    fixture.tree.fail_next_event_reconcile();
    refresh(&mut fixture.tree, Vec::new(), Vec::new());

    assert!(!fixture.tree.events.is_gated());
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(!fixture.tree.descendant_observation_is_settled());
    refresh(&mut fixture.tree, Vec::new(), Vec::new());

    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Complete);
    assert!(fixture.tree.descendant_observation_is_settled());
}

#[test]
fn gate_loss_after_complete_restarts_then_converges_by_fresh_dense_inventory() {
    let mut fixture = released_discovery_tree();
    refresh(&mut fixture.tree, Vec::new(), Vec::new());
    refresh(&mut fixture.tree, Vec::new(), Vec::new());
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Complete);

    fixture.tree.fail_next_event_read();
    refresh(&mut fixture.tree, Vec::new(), Vec::new());
    assert!(!fixture.tree.events.is_gated());
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Pending);

    refresh(&mut fixture.tree, Vec::new(), Vec::new());
    assert_eq!(fixture.tree.discovery_epoch, DiscoveryScanEpoch::Complete);
    assert!(fixture.tree.descendant_observation_is_settled());
}

#[test]
fn nonempty_dense_poll_remains_fail_closed() {
    let root = TestProcess::spawn();
    let child = TestProcess::spawn();
    let mut tree = exited_proof_tree(&root);
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let child_entry = entry(child.identity, root.identity.pid, child.identity.pid);
    tree.fail_next_event_reconcile();
    let entries = vec![root_entry, child_entry];
    refresh(&mut tree, entries.clone(), entries);

    assert_pending_fallback(&tree);
}

#[test]
fn indeterminate_dense_poll_remains_fail_closed() {
    let root = TestProcess::spawn();
    let child = TestProcess::spawn();
    let mut tree = exited_proof_tree(&root);
    let child_entry = entry(child.identity, root.identity.pid, child.identity.pid);
    tree.fail_next_event_reconcile();
    refresh(&mut tree, vec![child_entry], Vec::new());

    assert_pending_fallback(&tree);
}

#[test]
fn capability_pending_dense_poll_remains_fail_closed() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = test_support::transfer(issuer).unwrap();
    tree.install_process_capability(reader).unwrap();
    drop(writer);
    tree.begin_discovery_cleanup_internal();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    refresh(&mut tree, vec![root_entry], vec![root_entry]);
    tree.mark_root_exited_unreaped(tree.root_exit_proof_for_test(root.identity))
        .unwrap();
    tree.fail_next_event_reconcile();
    refresh(&mut tree, Vec::new(), Vec::new());

    assert!(tree.capability_scan_pending);
    assert_pending_fallback(&tree);
}

fn exited_proof_tree(root: &TestProcess) -> CapturedProcessTree {
    let mut tree = CapturedProcessTree::capture_with_discovery_internal(
        root.identity,
        TEST_CAPABILITY.to_owned(),
    )
    .unwrap();
    let proof = tree.root_exit_proof_for_test(root.identity);
    tree.mark_root_exited_unreaped(proof).unwrap();
    tree
}

fn released_discovery_tree() -> DiscoveryTreeTestFixture {
    let mut fixture = DiscoveryTreeTestFixture::spawn(TEST_CAPABILITY);
    fixture.exit_root();
    fixture.wait_for_kernel_eof();
    fixture
}

fn refresh(
    tree: &mut CapturedProcessTree,
    first: Vec<crate::process::inventory_snapshot::InventoryEntry>,
    second: Vec<crate::process::inventory_snapshot::InventoryEntry>,
) {
    let _inventory = install_test_inventory([first, second]);
    tree.refresh().unwrap();
}

fn assert_pending_fallback(tree: &CapturedProcessTree) {
    assert!(!tree.events.is_gated());
    assert_eq!(tree.discovery_epoch, DiscoveryScanEpoch::Pending);
    assert!(!tree.descendant_observation_is_settled());
    assert!(tree.lineage_integrity_lost || tree.capability_scan_pending);
}
