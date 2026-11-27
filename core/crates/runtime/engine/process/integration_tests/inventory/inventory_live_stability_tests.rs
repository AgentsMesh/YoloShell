use super::PtyProcessScope;
use super::inventory_snapshot::{
    InventoryEntry, install_test_inventory, install_test_inventory_results,
};
use super::inventory_stable::StableInventory;
use super::inventory_test_support::{TestProcess, entry};
use std::io;

const ROOT_PID: u32 = 100;
const CHILD_PID: u32 = 101;
const FIRST_PARENT_PID: u32 = 102;
const SECOND_PARENT_PID: u32 = 103;
const ROOT_UNIQUE_ID: u64 = 500;
const CHILD_UNIQUE_ID: u64 = 501;
const SECOND_UNIQUE_ID: u64 = 502;
const FIRST_PARENT_UNIQUE_ID: u64 = 503;
const SECOND_PARENT_UNIQUE_ID: u64 = 504;
const START_TIME: u64 = 600;
const PROCESS_GROUP: u32 = 300;
const SESSION_ID: u32 = 400;
const CHANGED_PROCESS_GROUP: u32 = 301;
const CHANGED_SESSION_ID: u32 = 401;
const FORGED_UNIQUE_ID: u64 = u64::MAX;
const RETRIED_SNAPSHOT_FAILURE_DETAIL: &str = "injected retried inventory failure";

#[test]
fn pid_reuse_between_snapshots_is_filtered() {
    let first = synthetic(ROOT_PID, ROOT_UNIQUE_ID, 0, 0);
    let mut replacement = first;
    replacement.unique_id = SECOND_UNIQUE_ID;
    let _inventory = install_test_inventory([vec![first], vec![replacement]]);

    assert!(StableInventory::capture().unwrap().entries().is_empty());
}

#[test]
fn retry_cannot_splice_two_process_generations_into_one_snapshot() {
    let first = synthetic(ROOT_PID, ROOT_UNIQUE_ID, 0, 0);
    let mut replacement = first;
    replacement.unique_id = SECOND_UNIQUE_ID;
    let _inventory = install_test_inventory_results([
        Ok(vec![first]),
        Err(io::Error::other(RETRIED_SNAPSHOT_FAILURE_DETAIL)),
        Ok(vec![replacement]),
    ]);

    assert!(StableInventory::capture().unwrap().entries().is_empty());
}

#[test]
fn ordinary_reparent_with_the_same_parent_unique_id_is_stable() {
    let first = synthetic(
        CHILD_PID,
        CHILD_UNIQUE_ID,
        FIRST_PARENT_PID,
        FIRST_PARENT_UNIQUE_ID,
    );
    let mut reparented = first;
    reparented.parent_pid = SECOND_PARENT_PID;
    let _inventory = install_test_inventory([vec![first], vec![reparented]]);

    let stable = StableInventory::capture().unwrap();

    assert_eq!(stable.entries(), [first]);
}

#[test]
fn parent_unique_id_change_is_filtered_then_accepted_when_stable() {
    let first = synthetic(
        CHILD_PID,
        CHILD_UNIQUE_ID,
        FIRST_PARENT_PID,
        FIRST_PARENT_UNIQUE_ID,
    );
    let mut changed = first;
    changed.parent_pid = SECOND_PARENT_PID;
    changed.parent_unique_id = SECOND_PARENT_UNIQUE_ID;
    let _inventory =
        install_test_inventory([vec![first], vec![changed], vec![changed], vec![changed]]);

    assert!(StableInventory::capture().unwrap().entries().is_empty());
    assert_eq!(StableInventory::capture().unwrap().entries(), [changed]);
}

#[test]
fn process_group_and_session_changes_are_filtered() {
    let first = synthetic(CHILD_PID, CHILD_UNIQUE_ID, ROOT_PID, ROOT_UNIQUE_ID);
    let mut changed_group = first;
    changed_group.process_group = CHANGED_PROCESS_GROUP;
    assert_change_is_filtered(first, changed_group);
    let mut changed_session = first;
    changed_session.session_id = CHANGED_SESSION_ID;
    assert_change_is_filtered(first, changed_session);
}

#[test]
fn durable_root_rejects_a_forged_live_unique_id() {
    let root = TestProcess::spawn();
    let mut forged = entry(root.identity, 0, root.identity.pid);
    forged.unique_id = FORGED_UNIQUE_ID;
    let _inventory = install_test_inventory([vec![forged], vec![forged]]);

    assert!(PtyProcessScope::capture(root.identity).is_err());
    assert!(root.identity.matches());
}

#[test]
fn duplicate_and_self_parent_snapshots_fail_closed() {
    let valid = synthetic(CHILD_PID, CHILD_UNIQUE_ID, ROOT_PID, ROOT_UNIQUE_ID);
    assert_snapshot_fails(vec![valid, valid], vec![valid]);
    assert_snapshot_fails(vec![valid], vec![valid, valid]);
    let mut same_pid = valid;
    same_pid.unique_id = SECOND_UNIQUE_ID;
    assert_snapshot_fails(vec![valid, same_pid], vec![valid]);
    let mut same_unique = valid;
    same_unique.pid = ROOT_PID;
    assert_snapshot_fails(vec![valid, same_unique], vec![valid]);
    let mut self_parent = valid;
    self_parent.parent_unique_id = CHILD_UNIQUE_ID;
    assert_snapshot_fails(vec![self_parent], vec![self_parent]);
}

fn assert_change_is_filtered(first: InventoryEntry, changed: InventoryEntry) {
    let _inventory = install_test_inventory([vec![first], vec![changed]]);
    assert!(StableInventory::capture().unwrap().entries().is_empty());
}

fn assert_snapshot_fails(first: Vec<InventoryEntry>, second: Vec<InventoryEntry>) {
    let _inventory = install_test_inventory([first, second]);
    assert!(StableInventory::capture().is_err());
}

fn synthetic(pid: u32, unique_id: u64, parent_pid: u32, parent_unique_id: u64) -> InventoryEntry {
    InventoryEntry {
        pid,
        parent_pid,
        process_group: PROCESS_GROUP,
        session_id: SESSION_ID,
        start_time: START_TIME,
        unique_id,
        parent_unique_id,
    }
}
