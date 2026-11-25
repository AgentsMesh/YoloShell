use super::CapturedProcessTree;
use super::constants::EVENT_RECONCILIATION_LIMIT;
use super::inventory_snapshot::{InventoryEntry, install_test_inventory};
use super::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};

const EXTRA_WATCH_PROCESS_COUNT: usize = 1;
const NEW_WATCH_CLOSURE_PASSES: usize = 2;
const EVENT_TEST_PROCESS_COUNT: usize = EVENT_RECONCILIATION_LIMIT + EXTRA_WATCH_PROCESS_COUNT;

#[test]
fn event_at_reconciliation_limit_remains_pending_for_the_next_tick() {
    let root = TestProcess::spawn();
    let children = std::iter::repeat_with(TestProcess::spawn)
        .take(EVENT_TEST_PROCESS_COUNT)
        .collect::<Vec<_>>();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let mut visible = vec![root_entry];
    let mut snapshots = Vec::new();
    for child in children.iter().take(EVENT_RECONCILIATION_LIMIT) {
        visible.push(entry(child.identity, root.identity.pid, child.identity.pid));
        snapshots.extend([visible.clone(), visible.clone()]);
    }
    let final_child = children.last().unwrap();
    visible.push(entry(
        final_child.identity,
        root.identity.pid,
        final_child.identity.pid,
    ));
    for _closure in std::iter::repeat_n((), NEW_WATCH_CLOSURE_PASSES) {
        snapshots.extend([visible.clone(), visible.clone()]);
    }
    let _inventory = install_test_inventory(snapshots);
    tree.invalidate_events();

    assert!(tree.refresh_if_changed().unwrap());
    let first_tick_last = children
        .iter()
        .take(EVENT_RECONCILIATION_LIMIT)
        .last()
        .unwrap();
    assert!(tree.identities().contains(&first_tick_last.identity));
    assert!(!tree.identities().contains(&final_child.identity));

    assert!(tree.refresh_if_changed().unwrap());
    assert!(tree.identities().contains(&final_child.identity));
}

#[test]
fn event_read_failure_degrades_to_dense_polling_without_tree_failure() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    tree.fail_next_event_read();

    assert!(refresh_if_changed(&mut tree, vec![root_entry]));
    assert!(refresh_if_changed(&mut tree, vec![root_entry]));
    assert_eq!(tree.identities(), [root.identity]);
}

#[test]
fn watcher_reconcile_failure_keeps_captured_targets_and_uses_dense_polling() {
    let root = TestProcess::spawn();
    let child = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let entries = vec![
        entry(root.identity, NO_PARENT_PID, root.identity.pid),
        entry(child.identity, root.identity.pid, child.identity.pid),
    ];
    tree.fail_next_event_reconcile();

    refresh(&mut tree, entries.clone());
    assert!(tree.identities().contains(&child.identity));
    assert!(refresh_if_changed(&mut tree, entries));
}

#[test]
fn sparse_audit_forces_inventory_without_a_kernel_event() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    tree.expire_event_audit();

    assert!(refresh_if_changed(&mut tree, vec![root_entry]));
}

#[test]
fn pending_capability_scan_retries_once_without_an_event_or_audit_expiry() {
    let root = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    tree.set_capability_scan_pending_for_test();

    assert!(tree.capability_scan_is_pending_for_test());
    assert!(refresh_if_changed(&mut tree, vec![root_entry]));
    assert!(!tree.capability_scan_is_pending_for_test());
    assert!(!tree.refresh_if_changed().unwrap());
}

fn refresh(tree: &mut CapturedProcessTree, entries: Vec<InventoryEntry>) {
    let _inventory = install_test_inventory([entries.clone(), entries]);
    tree.refresh().unwrap();
}

fn refresh_if_changed(tree: &mut CapturedProcessTree, entries: Vec<InventoryEntry>) -> bool {
    let _inventory = install_test_inventory([entries.clone(), entries]);
    tree.refresh_if_changed().unwrap()
}
