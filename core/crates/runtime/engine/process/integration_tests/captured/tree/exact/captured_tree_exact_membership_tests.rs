use crate::process::CapturedProcessTree;
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
use crate::process::inventory_snapshot::install_repeating_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};

#[test]
fn synthetic_session_membership_cannot_resolve_root_fork_debt() {
    let mut root = TestProcess::spawn();
    let mut target = TestProcess::spawn();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_live = tree.live_root();
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let target_entry = entry(target.identity, NO_PARENT_PID, root.identity.pid);
    let mut inventory = install_repeating_test_inventory(vec![root_entry, target_entry]);

    tree.events.inject_unresolved_fork(root_live);
    tree.refresh().unwrap();
    assert!(tree.identities().contains(&target.identity));

    tree.events.queue_test_exit(root_live);
    root.terminate();
    inventory.replace_repeating(vec![target_entry]);
    settle_events(&mut tree);

    assert_eq!(tree.request_force_termination().unwrap(), [target.identity]);
    assert!(tree.exact_lineage_containment.is_none());
    target.terminate();
}

fn settle_events(tree: &mut CapturedProcessTree) {
    for _attempt in 0..EVENT_RECONCILIATION_LIMIT {
        tree.refresh().unwrap();
        if !tree.events.requires_reconciliation() {
            return;
        }
    }
}
