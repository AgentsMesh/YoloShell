use super::CapturedProcessTree;
use super::inventory_snapshot::{InventoryEntry, install_test_inventory};
use super::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};

const UNRELATED_PARENT_PID: u32 = u32::MAX;
const UNRELATED_PARENT_UNIQUE_ID: u64 = u64::MAX;

#[test]
fn observed_intermediate_owns_a_reparented_setsid_leaf() {
    let (root, mut tree) = tree();
    let intermediate = TestProcess::spawn();
    let leaf = TestProcess::spawn();
    refresh(
        &mut tree,
        vec![root_entry(&root), owned(&intermediate, &root)],
    );
    assert!(tree.identities().contains(&intermediate.identity));

    refresh(
        &mut tree,
        vec![root_entry(&root), owned(&leaf, &intermediate)],
    );

    assert!(tree.identities().contains(&leaf.identity));
    assert!(leaf.identity.matches());
}

#[test]
fn numeric_parent_collision_without_unique_id_proof_is_rejected() {
    let (root, mut tree) = tree();
    let victim = TestProcess::spawn();
    let mut collision = unrelated(&victim);
    collision.parent_pid = root.identity.pid;

    refresh(&mut tree, vec![root_entry(&root), collision]);

    assert!(!tree.identities().contains(&victim.identity));
    assert!(victim.identity.matches());
}

#[test]
fn never_observed_intermediate_is_not_invented() {
    let (root, mut tree) = tree();
    let leaf = TestProcess::spawn();

    refresh(&mut tree, vec![root_entry(&root), unrelated(&leaf)]);

    assert_eq!(tree.identities(), [root.identity]);
    assert!(leaf.identity.matches());
}

#[test]
fn parent_change_after_capture_is_legal_and_new_children_still_attach() {
    let (root, mut tree) = tree();
    let child = TestProcess::spawn();
    let descendant = TestProcess::spawn();
    refresh(&mut tree, vec![root_entry(&root), owned(&child, &root)]);
    let mut changed_child = unrelated(&child);
    changed_child.parent_unique_id = UNRELATED_PARENT_UNIQUE_ID;
    refresh(
        &mut tree,
        vec![root_entry(&root), changed_child, owned(&descendant, &child)],
    );

    assert!(tree.identities().contains(&child.identity));
    assert!(tree.identities().contains(&descendant.identity));
}

#[test]
fn unowned_history_cannot_expand_future_ownership() {
    let (root, mut tree) = tree();
    let newly_owned = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let first_victim = owned(&victim, &newly_owned);
    refresh(
        &mut tree,
        vec![root_entry(&root), unrelated(&newly_owned), first_victim],
    );
    assert!(!tree.identities().contains(&victim.identity));

    refresh(
        &mut tree,
        vec![
            root_entry(&root),
            owned(&newly_owned, &root),
            unrelated(&victim),
        ],
    );

    assert!(tree.identities().contains(&newly_owned.identity));
    assert!(!tree.identities().contains(&victim.identity));
}

#[test]
fn ownership_fault_keeps_exact_lineage_but_disables_session_only_claims() {
    let (root, mut tree) = tree();
    let descendant = TestProcess::spawn();
    let session_collision = TestProcess::spawn();
    tree.mark_ownership_fault();
    let mut collision = unrelated(&session_collision);
    collision.session_id = root.identity.pid;

    refresh(
        &mut tree,
        vec![root_entry(&root), owned(&descendant, &root), collision],
    );

    assert!(tree.identities().contains(&descendant.identity));
    assert!(!tree.identities().contains(&session_collision.identity));
}

#[test]
fn retired_tree_does_not_acquire_new_exact_descendants() {
    let (root, mut tree) = tree();
    let descendant = TestProcess::spawn();
    tree.retire_session_for_test();
    let _inventory = install_test_inventory([
        vec![root_entry(&root), owned(&descendant, &root)],
        vec![root_entry(&root), owned(&descendant, &root)],
    ]);

    assert_eq!(tree.refresh().unwrap(), [root.identity]);
    assert!(descendant.identity.matches());
}

#[test]
fn idle_event_gate_skips_dense_inventory_refresh() {
    let (_root, mut tree) = tree();
    let _inventory = install_test_inventory([]);

    assert!(!tree.refresh_if_changed().unwrap());
}

fn tree() -> (TestProcess, CapturedProcessTree) {
    let root = TestProcess::spawn();
    let tree = CapturedProcessTree::capture(root.identity).unwrap();
    assert_eq!(tree.identities(), [root.identity]);
    (root, tree)
}

fn refresh(tree: &mut CapturedProcessTree, entries: Vec<InventoryEntry>) {
    let _inventory = install_test_inventory([entries.clone(), entries]);
    tree.refresh().unwrap();
}

fn root_entry(root: &TestProcess) -> InventoryEntry {
    entry(root.identity, NO_PARENT_PID, root.identity.pid)
}

fn owned(process: &TestProcess, parent: &TestProcess) -> InventoryEntry {
    entry(process.identity, parent.identity.pid, process.identity.pid)
}

fn unrelated(process: &TestProcess) -> InventoryEntry {
    let mut entry = entry(process.identity, NO_PARENT_PID, process.identity.pid);
    entry.parent_pid = UNRELATED_PARENT_PID;
    entry.parent_unique_id = UNRELATED_PARENT_UNIQUE_ID;
    entry
}
