use super::CapturedProcessTree;
use super::inventory_snapshot::{InventoryEntry, install_test_inventory};
use super::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};

const CHANGED_PROCESS_GROUP: u32 = u32::MAX;
const TWO_RETIRED_ANCHORS: usize = 2;
const ONE_RETIRED_ANCHOR: usize = 1;

#[test]
fn unstable_child_keeps_a_dead_parent_anchor_until_capture() {
    let root = TestProcess::spawn();
    let mut parent = TestProcess::spawn();
    let child = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let parent_entry = entry(parent.identity, root.identity.pid, parent.identity.pid);
    let child_entry = entry(child.identity, parent.identity.pid, child.identity.pid);
    refresh_stable(&mut tree, [root_entry, parent_entry]);
    assert!(tree.identities().contains(&parent.identity));
    parent.terminate();

    refresh_snapshots(&mut tree, vec![root_entry], vec![root_entry, child_entry]);
    assert!(!tree.identities().contains(&parent.identity));
    assert!(!tree.identities().contains(&child.identity));
    assert!(!tree.descendant_observation_is_settled());

    let mut changed_child = child_entry;
    changed_child.process_group = CHANGED_PROCESS_GROUP;
    refresh_snapshots(
        &mut tree,
        vec![root_entry, child_entry],
        vec![root_entry, changed_child],
    );
    assert!(!tree.identities().contains(&parent.identity));
    assert!(!tree.identities().contains(&child.identity));
    assert!(!tree.descendant_observation_is_settled());

    refresh_stable(&mut tree, [root_entry, child_entry]);
    assert!(!tree.identities().contains(&parent.identity));
    assert!(tree.identities().contains(&child.identity));
    assert!(!tree.descendant_observation_is_settled());

    refresh_stable(&mut tree, [root_entry, child_entry]);
    assert!(tree.descendant_observation_is_settled());
}

#[test]
fn unrelated_churn_does_not_retain_a_settled_tombstone() {
    let root = TestProcess::spawn();
    let mut settled_parent = TestProcess::spawn();
    let mut busy_parent = TestProcess::spawn();
    let child = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let settled_entry = entry(
        settled_parent.identity,
        root.identity.pid,
        settled_parent.identity.pid,
    );
    let busy_entry = entry(
        busy_parent.identity,
        root.identity.pid,
        busy_parent.identity.pid,
    );
    let child_entry = entry(child.identity, busy_parent.identity.pid, child.identity.pid);
    refresh_stable(&mut tree, [root_entry, settled_entry, busy_entry]);
    settled_parent.terminate();
    busy_parent.terminate();
    refresh_stable(&mut tree, [root_entry]);
    assert_eq!(tree.retiring_count(), TWO_RETIRED_ANCHORS);

    refresh_snapshots(&mut tree, vec![root_entry], vec![root_entry, child_entry]);

    assert_eq!(tree.retiring_count(), ONE_RETIRED_ANCHOR);
    assert!(!tree.descendant_observation_is_settled());
}

#[test]
fn conflicting_parent_evidence_blocks_then_resolves_without_signal_authority() {
    let root = TestProcess::spawn();
    let outsider = TestProcess::spawn();
    let child = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let owned_edge = entry(child.identity, root.identity.pid, child.identity.pid);
    let foreign_edge = entry(child.identity, outsider.identity.pid, child.identity.pid);

    refresh_snapshots(
        &mut tree,
        vec![root_entry, owned_edge],
        vec![root_entry, foreign_edge],
    );
    assert!(!tree.identities().contains(&child.identity));
    assert!(!tree.descendant_observation_is_settled());

    refresh_stable(&mut tree, [root_entry, foreign_edge]);
    assert!(!tree.identities().contains(&child.identity));
    assert!(tree.descendant_observation_is_settled());

    refresh_stable(&mut tree, [root_entry, owned_edge]);
    assert!(tree.identities().contains(&child.identity));
}

#[test]
fn session_only_unstable_member_bridges_a_later_setsid_leaf() {
    let root = TestProcess::spawn();
    let intermediate = TestProcess::spawn();
    let leaf = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let intermediate_entry = entry(intermediate.identity, NO_PARENT_PID, root.identity.pid);
    let leaf_entry = entry(leaf.identity, intermediate.identity.pid, leaf.identity.pid);

    refresh_snapshots(
        &mut tree,
        vec![root_entry],
        vec![root_entry, intermediate_entry, leaf_entry],
    );
    assert!(!tree.identities().contains(&leaf.identity));
    assert!(!tree.descendant_observation_is_settled());

    refresh_stable(&mut tree, [root_entry, leaf_entry]);
    assert!(tree.identities().contains(&leaf.identity));
}

fn refresh_stable<const COUNT: usize>(
    tree: &mut CapturedProcessTree,
    entries: [InventoryEntry; COUNT],
) {
    let entries = Vec::from(entries);
    refresh_snapshots(tree, entries.clone(), entries);
}

fn refresh_snapshots(
    tree: &mut CapturedProcessTree,
    first: Vec<InventoryEntry>,
    second: Vec<InventoryEntry>,
) {
    let _inventory = install_test_inventory([first, second]);
    tree.refresh().unwrap();
}
