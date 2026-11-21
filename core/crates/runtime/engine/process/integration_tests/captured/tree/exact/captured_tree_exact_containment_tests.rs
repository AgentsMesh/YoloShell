use crate::process::captured_targets::{install_test_signal_failure, install_test_signal_noop};
use crate::process::constants::EVENT_RECONCILIATION_LIMIT;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::{CapturedProcessTree, LiveProcessIdentity};

const SIGNAL_FAILURE_DETAIL: &str = "injected exact-containment signal failure";

#[test]
fn empty_force_targets_cannot_authorize_preexisting_fork_debt() {
    let (mut root, mut tree, root_live) = captured_root();
    tree.events.inject_unresolved_fork(root_live);
    root.terminate();
    settle_after_exit(&mut tree);
    assert!(tree.captured_targets().is_empty());

    assert!(tree.request_force_termination().unwrap().is_empty());
    assert!(tree.exact_lineage_containment.is_none());
    settle_publication(&mut tree);

    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [root_live.lineage_id()]
    );
}

#[test]
fn failed_sigkill_cannot_authorize_a_later_natural_exit() {
    let (mut root, mut tree, root_live) = captured_root();
    tree.events.inject_unresolved_fork(root_live);
    let failure =
        install_test_signal_failure(libc::SIGKILL, usize::default(), SIGNAL_FAILURE_DETAIL);

    assert!(tree.request_force_termination().is_err());
    assert!(tree.exact_lineage_containment.is_none());
    drop(failure);
    root.terminate();
    settle_after_exit(&mut tree);

    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [root_live.lineage_id()]
    );
}

#[test]
fn ignored_sigterm_cannot_authorize_a_later_fork_exit() {
    let (mut root, mut tree, root_live) = captured_root();
    tree.events.inject_unresolved_fork(root_live);
    let _noop = install_test_signal_noop(libc::SIGTERM, usize::default());

    assert_eq!(
        tree.request_graceful_termination().unwrap(),
        [root.identity]
    );
    assert!(tree.exact_lineage_containment.is_none());
    tree.events.queue_test_fork_exit(root_live);
    root.terminate();
    settle_after_exit(&mut tree);

    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [root_live.lineage_id()]
    );
}

#[test]
fn killing_an_unrelated_target_cannot_authorize_another_lineage() {
    let (mut target, mut tree, _target_live) = captured_root();
    let mut source = TestProcess::spawn();
    let source_live = live_identity(&source);
    tree.events.inject_gone_unresolved_fork(source_live);

    assert_eq!(tree.request_force_termination().unwrap(), [target.identity]);
    assert!(tree.exact_lineage_containment.is_none());
    target.terminate();
    settle_after_exit(&mut tree);

    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [source_live.lineage_id()]
    );
    source.terminate();
}

#[test]
fn partial_sigkill_delivery_cannot_authorize_later_settlement() {
    let (mut root, mut tree, root_live) = captured_root();
    let mut child = TestProcess::spawn();
    capture_child(&mut tree, &root, &child);
    tree.events.inject_unresolved_fork(root_live);
    let root_index = tree
        .captured_targets()
        .iter()
        .position(|(identity, _durable)| identity == &root_live)
        .unwrap();
    let failure = install_test_signal_failure(libc::SIGKILL, root_index, SIGNAL_FAILURE_DETAIL);

    assert!(tree.request_force_termination().is_err());
    assert!(tree.exact_lineage_containment.is_none());
    drop(failure);
    root.terminate();
    child.terminate();
    settle_after_exit(&mut tree);

    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [root_live.lineage_id()]
    );
}

#[test]
fn disappearing_unstable_edge_cannot_authorize_force_containment() {
    let (mut root, mut tree, root_live) = captured_root();
    let mut target = TestProcess::spawn();
    let target_live = live_identity(&target);
    let root_entry = entry(root.identity, NO_PARENT_PID, root.identity.pid);
    let unstable_target = entry(target.identity, root.identity.pid, root.identity.pid);
    let stable_target = entry(target.identity, NO_PARENT_PID, root.identity.pid);
    let mut inventory = install_test_inventory([
        vec![root_entry, unstable_target],
        vec![root_entry, stable_target],
    ]);
    tree.events.inject_unresolved_fork(root_live);

    tree.refresh().unwrap();
    inventory.replace_repeating(vec![root_entry, stable_target]);
    tree.refresh().unwrap();
    assert!(tree.identities().contains(&target.identity));

    tree.events.queue_test_exit(root_live);
    root.terminate();
    inventory.replace_repeating(vec![stable_target]);
    settle_publication(&mut tree);
    assert!(!tree.events.requires_reconciliation());
    assert_eq!(
        tree.events.unresolved_fork_lineages_for_test(),
        [root_live.lineage_id()]
    );
    assert!(tree.events.exact_closure_isolated(&[target_live]));

    let killed = tree.request_force_termination().unwrap();
    assert_eq!(killed, [target.identity]);
    assert!(tree.exact_lineage_containment.is_none());
    target.terminate();
}

fn captured_root() -> (TestProcess, CapturedProcessTree, LiveProcessIdentity) {
    let root = TestProcess::spawn();
    let tree = CapturedProcessTree::capture(root.identity).unwrap();
    let root_live = tree.live_root();
    (root, tree, root_live)
}

fn live_identity(process: &TestProcess) -> LiveProcessIdentity {
    LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap()
}

fn capture_child(tree: &mut CapturedProcessTree, root: &TestProcess, child: &TestProcess) {
    let entries = vec![
        entry(root.identity, NO_PARENT_PID, root.identity.pid),
        entry(child.identity, root.identity.pid, child.identity.pid),
    ];
    let _inventory = install_test_inventory([entries.clone(), entries]);
    tree.refresh().unwrap();
    assert!(tree.identities().contains(&child.identity));
}

fn settle_after_exit(tree: &mut CapturedProcessTree) {
    settle_publication(tree);
    assert!(tree.captured_targets().is_empty());
}

fn settle_publication(tree: &mut CapturedProcessTree) {
    for _refresh in 0..EVENT_RECONCILIATION_LIMIT {
        tree.refresh().unwrap();
    }
}
