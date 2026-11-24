use super::CapturedProcessTree;
use crate::process::inventory_snapshot::install_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::live_identity::ProcessExecutionState;
use crate::process::{DestructiveSessionUnavailable, LiveProcessIdentity};

const INVENTORY_SNAPSHOTS: usize = 16;
const UNAUTHORIZED_FORCE_STARTED_DETAIL: &str = "force freeze started without signal authority";

#[test]
fn exited_root_with_live_descendant_still_requires_signal_authority() {
    let mut root = TestProcess::spawn();
    let descendant = TestProcess::spawn();
    let descendant_live = LiveProcessIdentity::capture(descendant.identity)
        .unwrap()
        .unwrap();
    let mut tree = CapturedProcessTree::capture(root.identity).unwrap();
    tree.authorize_destructive_for_test();
    let inventory = vec![
        entry(root.identity, NO_PARENT_PID, root.identity.pid),
        entry(descendant.identity, root.identity.pid, root.identity.pid),
    ];
    let _inventory = install_test_inventory(std::iter::repeat_n(inventory, INVENTORY_SNAPSHOTS));
    tree.refresh().unwrap();
    tree.refresh().unwrap();
    assert!(tree.identities().contains(&descendant.identity));

    let root_exit = tree.root_exit_proof_for_test(root.identity);
    root.terminate();
    tree.mark_root_exited_unreaped(root_exit).unwrap();
    tree.deny_destructive_for_test();

    let error = match tree.begin_force().poll() {
        Err(error) => error,
        Ok(_) => panic!("{UNAUTHORIZED_FORCE_STARTED_DETAIL}"),
    };

    assert!(error.error().is::<DestructiveSessionUnavailable>());
    assert_eq!(
        descendant_live.execution_state().unwrap(),
        ProcessExecutionState::Running
    );
}
