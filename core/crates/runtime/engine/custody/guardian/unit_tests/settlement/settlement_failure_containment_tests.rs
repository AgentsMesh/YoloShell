use super::*;
use crate::process::ProcessIdentityRelation;
use std::os::unix::process::ExitStatusExt;
use std::time::Instant;

#[path = "settlement_failure_discovery_tests.rs"]
mod discovery_tests;
#[path = "settlement_failure_drain_retry_tests.rs"]
mod drain_retry_tests;
#[path = "../../test_support/unit/settlement_failure_escaped_holder.rs"]
mod escaped_holder;
#[path = "settlement_failure_escaped_holder_tests.rs"]
mod escaped_holder_tests;
#[path = "settlement_failure_containment_fixture.rs"]
mod fixture;
#[path = "settlement_failure_late_holder.rs"]
mod late_holder;
#[path = "settlement_failure_late_holder_tests.rs"]
mod late_holder_tests;
#[path = "../../test_support/unit/settlement_failure_premature_fixture.rs"]
mod premature_fixture;
#[path = "settlement_failure_queued_holder.rs"]
mod queued_holder;
#[path = "settlement_failure_queued_holder_tests.rs"]
mod queued_holder_tests;
#[path = "../../test_support/unit/settlement_failure_stopped_child_tests.rs"]
mod stopped_child;
use fixture::{Fixture, TEST_POLL_INTERVAL, TEST_TIMEOUT};
use premature_fixture::PrematureFixture;

static REAL_PTY_CONTAINMENT_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
const CONTAINMENT_CONVERGENCE_DETAIL: &str =
    "namespace-loss containment did not converge after transient discovery errors";

#[test]
fn escaped_pty_holder_helper() {
    escaped_holder::run_role();
}

#[test]
fn namespace_loss_defers_active_custody_settlement_until_exact_tree_proof() {
    let _isolation = serialize_real_pty_containment_tests();
    let (mut fixture, _worker) = Fixture::new(ProcessIdentityRelation::Gone);
    let origin = tokio::time::Instant::now();
    let mut containment = super::super::settlement_containment::NamespaceLossContainment::new(
        &fixture.runtime,
        origin,
    );

    assert!(
        reconcile_before(
            &fixture.custody,
            &mut containment,
            origin,
            Instant::now() + TEST_TIMEOUT,
        )
        .is_none()
    );
    assert!(
        namespace_processes_settled(&fixture.custody, &fixture.runtime, None)
            .unwrap()
            .is_none()
    );
    fixture.assert_child_pending();
    fixture.assert_residual_matching();

    let graceful = origin + super::super::settlement_containment::NAMESPACE_LOSS_OBSERVATION_GRACE;
    assert!(
        reconcile_before(
            &fixture.custody,
            &mut containment,
            graceful,
            Instant::now() + TEST_TIMEOUT,
        )
        .is_none()
    );
    assert!(
        namespace_processes_settled(&fixture.custody, &fixture.runtime, None)
            .unwrap()
            .is_none()
    );
    fixture.assert_child_pending();
    fixture.assert_residual_matching();

    let force = graceful + super::super::settlement_containment::NAMESPACE_LOSS_GRACEFUL_GRACE;
    let _ = reconcile_before(
        &fixture.custody,
        &mut containment,
        force,
        Instant::now() + TEST_TIMEOUT,
    );
    assert!(containment.force_requested());
    let settled = wait_for_settlement(&fixture.custody, &fixture.runtime, &mut containment, force);
    assert_eq!(settled.child, child_identity(&fixture.runtime));
    assert_eq!(settled.worker, worker_identity(&fixture.runtime));
    assert_eq!(fixture.wait_for_child_exit().signal(), Some(libc::SIGKILL));
    assert_eq!(
        fixture.wait_for_residual_exit().signal(),
        Some(libc::SIGKILL)
    );
}

#[test]
fn child_root_exit_confirmation_is_not_delayed_by_a_live_worker() {
    let _isolation = serialize_real_pty_containment_tests();
    let (mut fixture, worker_gone) = Fixture::new(ProcessIdentityRelation::Gone);
    let origin = tokio::time::Instant::now();
    let mut containment = super::super::settlement_containment::NamespaceLossContainment::new(
        &fixture.runtime,
        origin,
    );

    containment
        .reconcile_custody(&fixture.custody, origin)
        .unwrap();
    fixture.assert_child_pending();
    drop(worker_gone);
    let worker = worker_identity(&fixture.runtime);
    let _worker_matching = crate::process::install_test_process_identity_relation(
        worker.pid,
        worker.start_time,
        ProcessIdentityRelation::Matching,
    );
    assert_eq!(fixture.terminate_residual().signal(), Some(libc::SIGKILL));
    assert!(!fixture.custody.closure_settled().unwrap());
    assert_eq!(fixture.terminate_child().signal(), Some(libc::SIGKILL));
    assert!(
        namespace_processes_settled(&fixture.custody, &fixture.runtime, None)
            .unwrap()
            .is_none()
    );
    wait_for_closure_settlement(&fixture);
}

#[test]
fn stopped_child_helper() {
    stopped_child::run();
}

fn wait_for_closure_settlement(fixture: &Fixture) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while !fixture.custody.closure_settled().unwrap() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn wait_for_settlement(
    custody: &MacosSecondaryCustody,
    runtime: &CustodyRuntimeIdentity,
    containment: &mut super::super::settlement_containment::NamespaceLossContainment,
    now: tokio::time::Instant,
) -> NamespaceRuntimeSettled {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        let exact = reconcile_before(custody, containment, now, deadline);
        if let Some(settled) = namespace_processes_settled(custody, runtime, exact).unwrap() {
            return settled;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn reconcile_before(
    custody: &MacosSecondaryCustody,
    containment: &mut super::super::settlement_containment::NamespaceLossContainment,
    now: tokio::time::Instant,
    deadline: Instant,
) -> Option<super::super::settlement_containment::NamespaceLossExactSettlement> {
    loop {
        match containment.reconcile_custody(custody, now) {
            Ok(exact) => return exact,
            Err(error) => assert!(
                Instant::now() < deadline,
                "{CONTAINMENT_CONVERGENCE_DETAIL}: {error:#}"
            ),
        }
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

fn serialize_real_pty_containment_tests() -> std::sync::MutexGuard<'static, ()> {
    REAL_PTY_CONTAINMENT_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
