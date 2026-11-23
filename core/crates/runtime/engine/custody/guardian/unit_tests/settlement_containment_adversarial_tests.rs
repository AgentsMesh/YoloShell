use super::*;
use crate::process::{MacosProcessCapabilityIssuer, ProcessIdentityObservation};
use std::os::unix::process::ExitStatusExt;

#[path = "../../test_support/unit/settlement_non_holder_tree.rs"]
mod tree;

const TEST_WORKER_PID: u32 = 48_004;
const TEST_WORKER_START: u64 = 58_004;
const EXACT_SETTLEMENT_TIMEOUT_DETAIL: &str = "exact non-holder tree settlement was not proven";
const FORCE_REFRESH_FAILURE_DETAIL: &str = "injected Guardian force refresh failure";

#[test]
fn captured_non_holder_term_ignoring_descendant_must_be_observed_gone() {
    let capability = MacosProcessCapabilityIssuer::create().unwrap();
    let writer_handle = capability.identity().writer_handle();
    let mut tree = tree::NonHolderTree::spawn();
    let root = tree.root_identity();
    let leaf = tree.wait_for_leaf();
    assert_non_holder(root, writer_handle);
    assert_non_holder(leaf, writer_handle);
    let runtime = runtime(root);
    let _worker = crate::process::install_test_process_identity_relation(
        TEST_WORKER_PID,
        TEST_WORKER_START,
        crate::process::ProcessIdentityRelation::Gone,
    );
    let origin = Instant::now();
    let mut containment = NamespaceLossContainment::new(&runtime, origin);

    assert!(!containment.reconcile_for_test(origin).unwrap());
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    assert!(!containment.reconcile_for_test(graceful).unwrap());
    assert_eq!(root.observe(), ProcessIdentityObservation::Matching);
    assert_eq!(leaf.observe(), ProcessIdentityObservation::Matching);

    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;
    assert!(!containment.reconcile_for_test(force).unwrap());
    assert!(containment.force_requested());
    assert_eq!(tree.wait_for_root_exit().signal(), Some(libc::SIGKILL));
    assert!(tree.wait_for_leaf_exit());
    assert!(wait_for_exact_settlement(&mut containment, force));
}

#[test]
fn escaped_non_holder_descendant_helper() {
    tree::run_role();
}

#[test]
fn force_refresh_failure_withholds_exact_settlement_until_retry() {
    let mut tree = tree::NonHolderTree::spawn();
    let root = tree.root_identity();
    let _leaf = tree.wait_for_leaf();
    let runtime = runtime(root);
    let _worker = crate::process::install_test_process_identity_relation(
        TEST_WORKER_PID,
        TEST_WORKER_START,
        crate::process::ProcessIdentityRelation::Gone,
    );
    let origin = Instant::now();
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;
    let mut containment = NamespaceLossContainment::new(&runtime, origin);
    containment.reconcile_for_test(origin).unwrap();
    containment.reconcile_for_test(graceful).unwrap();
    containment
        .child
        .fail_next_refresh_for_test(FORCE_REFRESH_FAILURE_DETAIL);

    assert!(containment.reconcile_for_test(force).is_err());
    assert!(!containment.force_requested());

    let exact = containment.reconcile_for_test(force).unwrap();
    assert!(containment.force_requested());
    assert_eq!(tree.wait_for_root_exit().signal(), Some(libc::SIGKILL));
    assert!(tree.wait_for_leaf_exit());
    assert!(
        exact || wait_for_exact_settlement(&mut containment, force)
    );
}

fn assert_non_holder(identity: ProcessIdentity, writer_handle: u64) {
    assert!(identity.matches());
    assert!(!crate::process::process_holds_capability_for_test(identity.pid, writer_handle).unwrap());
}

fn runtime(child: ProcessIdentity) -> CustodyRuntimeIdentity {
    CustodyRuntimeIdentity {
        terminal_id: uuid::Uuid::new_v4(),
        runtime_epoch: uuid::Uuid::new_v4(),
        worker_id: uuid::Uuid::new_v4(),
        worker_pid: TEST_WORKER_PID,
        worker_start_time: TEST_WORKER_START,
        child_pid: child.pid,
        child_start_time: child.start_time,
        boot_id: crate::process::boot_id(),
    }
}

fn wait_for_exact_settlement(
    containment: &mut NamespaceLossContainment,
    now: Instant,
) -> bool {
    let deadline = std::time::Instant::now() + crate::session_timing::FORCE_TERMINATION_CONFIRM_TIMEOUT;
    loop {
        let settled = containment.reconcile_for_test(now).unwrap();
        if settled {
            return true;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "{EXACT_SETTLEMENT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL);
    }
}
