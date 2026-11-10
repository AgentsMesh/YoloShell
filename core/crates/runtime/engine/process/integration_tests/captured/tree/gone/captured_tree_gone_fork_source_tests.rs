use super::exact_gate_authority_tests::{pty_size, settle_exact_coverage, wait_for_stop};
use super::exact_stopped_closure;
use crate::process::inventory_snapshot::install_repeating_test_inventory;
use crate::process::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use crate::process::{
    CapturedProcessTree, LiveProcessIdentity, MacosProcessCapabilityIssuer,
    PTY_SESSION_CAPABILITY_ENV, PtySessionProof,
};
use portable_pty::native_pty_system;

const TEST_CAPABILITY: &str = "gone-fork-source-exact-authority";

#[test]
fn exited_descendant_fork_source_cannot_deadlock_exact_freeze() {
    let pair = native_pty_system().openpty(pty_size()).unwrap();
    let issuer = MacosProcessCapabilityIssuer::create().unwrap();
    let (reader, writer) = issuer.transfer_for_test().unwrap();
    let root = holder(&writer);
    let mut source = holder(&writer);
    let source_live = LiveProcessIdentity::capture(source.identity)
        .unwrap()
        .unwrap();
    drop(writer);
    let initial = vec![process_entry(&root), process_entry(&source)];
    let _inventory = install_repeating_test_inventory(initial);
    let mut proof =
        PtySessionProof::capture_bound(pair.master.as_ref(), TEST_CAPABILITY.to_owned()).unwrap();
    proof.install_process_capability(reader).unwrap();
    let mut tree = CapturedProcessTree::capture_pty(root.identity, proof).unwrap();
    tree.begin_frozen_discovery();
    settle_exact_coverage(&mut tree);
    assert!(
        tree.captured_targets()
            .iter()
            .any(|(identity, _durable)| identity == &source_live)
    );
    tree.events.inject_unresolved_fork(source_live);

    source.terminate();
    tree.begin_frozen_discovery();
    settle_exact_coverage(&mut tree);
    tree.terminate_captured(libc::SIGSTOP).unwrap();
    wait_for_stop(&tree);
    let expected = tree
        .captured_targets()
        .into_iter()
        .map(|(identity, _durable)| identity)
        .collect::<Vec<_>>();

    assert!(!expected.contains(&source_live));
    assert!(tree.events.has_unresolved_fork());
    assert!(tree.events.exact_closure_isolated(&expected));
    assert_eq!(exact_stopped_closure(&mut tree).unwrap(), Some(expected));
    assert!(tree.events.has_unresolved_fork());
}

fn holder(writer: &crate::process::MacosProcessCapabilityChildWriter) -> TestProcess {
    TestProcess::spawn_with_environment_and_process_capability(
        PTY_SESSION_CAPABILITY_ENV,
        TEST_CAPABILITY,
        writer,
    )
}

fn process_entry(process: &TestProcess) -> crate::process::inventory_snapshot::InventoryEntry {
    entry(process.identity, NO_PARENT_PID, process.identity.pid)
}
