use super::inventory_snapshot::{InventoryEntry, install_test_inventory};
use super::inventory_test_support::{TestProcess, entry};
use super::{ProcessIdentity, PtyProcessScope};
use std::collections::HashSet;
const STALE_START_TIME_OFFSET: u64 = 1;
const ROOT_PARENT_PID: u32 = 0;
#[cfg(target_os = "macos")]
const REPLACED_PARENT_PID: u32 = u32::MAX;
#[cfg(target_os = "macos")]
const REPLACEMENT_UNIQUE_ID: u64 = u64::MAX;

#[test]
fn anchor_replacement_snapshot_cannot_own_a_live_target() {
    let root = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let scope = PtyProcessScope::capture(root.identity).unwrap();
    let first = vec![
        entry(root.identity, ROOT_PARENT_PID, root.identity.pid),
        entry(victim.identity, root.identity.pid, victim.identity.pid),
    ];
    let mut replacement = first[0];
    let mut replacement_child = first[1];
    replacement.start_time = replacement.start_time.wrapping_add(STALE_START_TIME_OFFSET);
    #[cfg(target_os = "macos")]
    {
        replacement.unique_id = REPLACEMENT_UNIQUE_ID;
        replacement_child.parent_unique_id = REPLACEMENT_UNIQUE_ID;
    }
    let second = vec![replacement, replacement_child];
    assert_victim_not_selected(scope, first, second, victim.identity);
}

#[test]
fn intermediate_identity_change_breaks_synthetic_ancestry() {
    let root = TestProcess::spawn();
    let intermediate = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let scope = PtyProcessScope::capture(root.identity).unwrap();
    let first = ancestry(&root, &intermediate, &victim);
    let mut changed = first[1];
    changed.start_time = changed.start_time.wrapping_add(STALE_START_TIME_OFFSET);
    let second = vec![first[0], changed, first[2]];
    assert_victim_not_selected(scope, first, second, victim.identity);
}

#[test]
#[cfg(target_os = "macos")]
fn numeric_reparent_preserves_immutable_parent_ancestry() {
    let root = TestProcess::spawn();
    let intermediate = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let scope = PtyProcessScope::capture(root.identity).unwrap();
    let first = ancestry(&root, &intermediate, &victim);
    let mut reparented = first[1];
    reparented.parent_pid = REPLACED_PARENT_PID;
    let second = vec![first[0], reparented, first[2]];
    let _inventory = install_test_inventory([first, second]);

    let identities = scope.identities(&Default::default()).unwrap();

    assert!(identities.contains(&victim.identity));
    assert!(victim.identity.matches());
}

#[test]
fn foreground_lookup_rejects_replaced_root_snapshot() {
    let root = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let scope = PtyProcessScope::capture(root.identity).unwrap();
    let first = vec![
        entry(root.identity, ROOT_PARENT_PID, root.identity.pid),
        entry(victim.identity, root.identity.pid, victim.identity.pid),
    ];
    let mut replacement = first[0];
    replacement.start_time = replacement.start_time.wrapping_add(STALE_START_TIME_OFFSET);
    let _inventory = install_test_inventory([first.clone(), vec![replacement, first[1]]]);
    assert!(scope.foreground_group(victim.identity.pid as i32).is_err());
    assert!(victim.identity.matches());
}

#[test]
fn captured_same_session_member_witnesses_ownership_after_root_exit() {
    let root = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let scope = PtyProcessScope::capture(root.identity).unwrap();
    let session = vec![entry(victim.identity, ROOT_PARENT_PID, root.identity.pid)];
    let _inventory = install_test_inventory([session.clone(), session]);

    let anchors = HashSet::from([victim.identity]);
    let identities = scope.identities(&anchors).unwrap();

    assert_eq!(identities, [victim.identity]);
}

#[test]
fn root_absence_without_a_live_witness_retires_numeric_session_ownership() {
    let root = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let scope = PtyProcessScope::capture(root.identity).unwrap();
    let session = vec![entry(victim.identity, ROOT_PARENT_PID, root.identity.pid)];
    let _inventory = install_test_inventory([session.clone(), session]);

    let identities = scope.identities(&Default::default()).unwrap();

    assert!(identities.is_empty());
    assert!(victim.identity.matches());
}

#[test]
#[cfg(target_os = "macos")]
fn reused_session_leader_pid_cannot_claim_an_unrelated_session() {
    let root = TestProcess::spawn();
    let victim = TestProcess::spawn();
    let scope = PtyProcessScope::capture(root.identity).unwrap();
    let mut replacement = entry(root.identity, ROOT_PARENT_PID, root.identity.pid);
    replacement.start_time = replacement.start_time.wrapping_add(STALE_START_TIME_OFFSET);
    replacement.unique_id = REPLACEMENT_UNIQUE_ID;
    let mut victim_entry = entry(victim.identity, replacement.pid, root.identity.pid);
    victim_entry.parent_unique_id = REPLACEMENT_UNIQUE_ID;
    let session = vec![replacement, victim_entry];
    let _inventory = install_test_inventory([session.clone(), session]);

    let identities = scope.identities(&Default::default()).unwrap();

    assert!(identities.is_empty());
    assert!(victim.identity.matches());
}

fn ancestry(
    root: &TestProcess,
    intermediate: &TestProcess,
    victim: &TestProcess,
) -> Vec<InventoryEntry> {
    vec![
        entry(root.identity, ROOT_PARENT_PID, root.identity.pid),
        entry(
            intermediate.identity,
            root.identity.pid,
            intermediate.identity.pid,
        ),
        entry(
            victim.identity,
            intermediate.identity.pid,
            victim.identity.pid,
        ),
    ]
}

fn assert_victim_not_selected(
    scope: PtyProcessScope,
    first: Vec<InventoryEntry>,
    second: Vec<InventoryEntry>,
    victim: ProcessIdentity,
) {
    let _inventory = install_test_inventory([first, second]);
    let targets = scope.identities(&Default::default()).unwrap();
    for target in targets
        .into_iter()
        .filter(|target| target.pid == victim.pid)
    {
        let _ = target.signal(libc::SIGKILL).unwrap();
    }
    assert!(victim.matches());
}
