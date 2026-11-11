use super::*;
use crate::process::{
    ProcessIdentityObservation, ProcessIdentityRelation, install_test_process_identity_relation,
    install_test_signal_failure, install_test_signal_noop,
};
use std::os::unix::process::ExitStatusExt;

#[path = "../test_support/unit/settlement_containment_support.rs"]
mod support;
use support::*;

const ROOT_SIGNAL_TARGET_INDEX: usize = usize::MIN;
const TERM_FAILURE_DETAIL: &str = "injected namespace-loss graceful containment failure";
const KILL_FAILURE_DETAIL: &str = "injected namespace-loss force containment failure";

#[test]
fn namespace_loss_escalates_stopped_exact_identities_after_named_graces() {
    let mut worker = ChildGuard::spawn_stopped_term_ignoring();
    let mut child = ChildGuard::spawn();
    let worker_identity = worker.identity;
    let child_identity = child.identity;
    let runtime = runtime(worker_identity, child_identity);
    let origin = Instant::now();
    let mut containment = NamespaceLossContainment::new(&runtime, origin);

    containment.reconcile_for_test(origin).unwrap();
    assert_eq!(
        worker_identity.observe(),
        ProcessIdentityObservation::Matching
    );
    assert_eq!(
        child_identity.observe(),
        ProcessIdentityObservation::Matching
    );

    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    containment.reconcile_for_test(graceful).unwrap();
    let child_status = child.wait_for_exit();
    assert_eq!(child_status.signal(), Some(libc::SIGTERM));
    assert_eq!(
        worker_identity.observe(),
        ProcessIdentityObservation::Matching
    );
    assert!(!containment.force_requested());

    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;
    let noop = install_test_signal_noop(libc::SIGKILL, ROOT_SIGNAL_TARGET_INDEX);
    let exact = containment.reconcile_for_test(force).unwrap();
    assert!(containment.force_requested());
    assert!(!exact);
    assert_eq!(
        worker_identity.observe(),
        ProcessIdentityObservation::Matching
    );
    drop(noop);
    containment.reconcile_for_test(force).unwrap();
    let worker_status = worker.wait_for_exit();

    assert_eq!(worker_status.signal(), Some(libc::SIGKILL));
    assert!(wait_for_exact_settlement(&mut containment, force));
    assert_eq!(worker_identity.observe(), ProcessIdentityObservation::Gone);
    assert_eq!(child_identity.observe(), ProcessIdentityObservation::Gone);
}

#[test]
fn graceful_failure_retains_authority_and_retries_before_starting_grace() {
    let mut worker = ChildGuard::spawn();
    let mut child = ChildGuard::spawn();
    let origin = Instant::now();
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    let mut containment =
        NamespaceLossContainment::new(&runtime(worker.identity, child.identity), origin);
    containment.reconcile_for_test(origin).unwrap();
    let _failure =
        install_test_signal_failure(libc::SIGTERM, ROOT_SIGNAL_TARGET_INDEX, TERM_FAILURE_DETAIL);

    assert!(containment.reconcile_for_test(graceful).is_err());
    assert!(!containment.force_requested());
    assert_eq!(
        worker.identity.observe(),
        ProcessIdentityObservation::Matching
    );
    assert_eq!(child.wait_for_exit().signal(), Some(libc::SIGTERM));

    containment.reconcile_for_test(graceful).unwrap();
    assert_eq!(worker.wait_for_exit().signal(), Some(libc::SIGTERM));
    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;
    containment.reconcile_for_test(force).unwrap();
    assert!(containment.force_requested());
}

#[test]
fn target_failure_does_not_starve_later_holder_containment() {
    let worker = ChildGuard::spawn();
    let mut child = ChildGuard::spawn();
    let mut holder = ChildGuard::spawn();
    let origin = Instant::now();
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    let mut containment =
        NamespaceLossContainment::new(&runtime(worker.identity, child.identity), origin);
    containment.holders.push(ExactTarget::new(holder.identity));
    containment.reconcile_for_test(origin).unwrap();
    let _failure =
        install_test_signal_failure(libc::SIGTERM, ROOT_SIGNAL_TARGET_INDEX, TERM_FAILURE_DETAIL);

    assert!(containment.reconcile_for_test(graceful).is_err());
    assert_eq!(child.wait_for_exit().signal(), Some(libc::SIGTERM));
    assert_eq!(holder.wait_for_exit().signal(), Some(libc::SIGTERM));
}

#[test]
fn force_failure_withholds_authority_until_exact_retry_succeeds() {
    let mut worker = ChildGuard::spawn_stopped_term_ignoring();
    let mut child = ChildGuard::spawn();
    let origin = Instant::now();
    let graceful = origin + NAMESPACE_LOSS_OBSERVATION_GRACE;
    let force = graceful + NAMESPACE_LOSS_GRACEFUL_GRACE;
    let mut containment =
        NamespaceLossContainment::new(&runtime(worker.identity, child.identity), origin);
    containment.reconcile_for_test(origin).unwrap();
    containment.reconcile_for_test(graceful).unwrap();
    assert_eq!(child.wait_for_exit().signal(), Some(libc::SIGTERM));
    let _failure =
        install_test_signal_failure(libc::SIGKILL, ROOT_SIGNAL_TARGET_INDEX, KILL_FAILURE_DETAIL);

    assert!(containment.reconcile_for_test(force).is_err());
    assert!(!containment.force_requested());
    assert_eq!(
        worker.identity.observe(),
        ProcessIdentityObservation::Matching
    );

    containment.reconcile_for_test(force).unwrap();
    assert!(containment.force_requested());
    assert_eq!(worker.wait_for_exit().signal(), Some(libc::SIGKILL));
}

#[test]
fn indeterminate_capture_retries_without_losing_the_exact_targets() {
    let worker = ChildGuard::spawn();
    let child = ChildGuard::spawn();
    let origin = Instant::now();
    let mut containment =
        NamespaceLossContainment::new(&runtime(worker.identity, child.identity), origin);
    let unknown = install_test_process_identity_relation(
        worker.identity.pid,
        worker.identity.start_time,
        ProcessIdentityRelation::Unknown,
    );

    assert!(containment.reconcile_for_test(origin).is_err());
    assert!(!containment.force_requested());
    drop(unknown);

    containment.reconcile_for_test(origin).unwrap();
    assert_eq!(
        worker.identity.observe(),
        ProcessIdentityObservation::Matching
    );
    assert_eq!(
        child.identity.observe(),
        ProcessIdentityObservation::Matching
    );
}

fn wait_for_exact_settlement(containment: &mut NamespaceLossContainment, now: Instant) -> bool {
    let deadline =
        std::time::Instant::now() + crate::session_timing::FORCE_TERMINATION_CONFIRM_TIMEOUT;
    loop {
        let settled = containment.reconcile_for_test(now).unwrap();
        if settled || std::time::Instant::now() >= deadline {
            return settled;
        }
        std::thread::sleep(crate::session_timing::GRACEFUL_TERMINATION_POLL_INTERVAL);
    }
}
