use super::process_assertions::{assert_process_tree_exits, bind_process_tree_inventory};
use super::tests::{
    TEST_MARKER_FILE, production_fixture, wait_for_descendant, wait_for_tracked_identity,
};
use super::{StartupChildGuard, StartupOutputProgress};
use crate::process::ProcessIdentity;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::time::{Duration, Instant};

const DROP_RETURN_DEADLINE: Duration = Duration::from_millis(250);
const DEFERRED_SETTLEMENT_TIMEOUT: Duration =
    crate::worker::termination_timing::WORKER_TERMINATION_TIMEOUT;
const MISSING_EOF_POLL_INTERVAL: Duration = Duration::from_millis(10);
const INJECTED_POST_COMMIT_PANIC: &str = "injected panic after bootstrap gate commit";
const PROCESS_SURVIVED_DETAIL: &str = "process survived deferred cleanup";
const SCOPE_SETTLEMENT_TIMEOUT_DETAIL: &str =
    "startup cleanup scope did not reach final fault settlement";
const EXCLUSIVE_OUTPUT_REQUIRED_DETAIL: &str = "production fixture retains exclusive PTY output";
const WAITPID_FAILURE: libc::pid_t = -1;

#[test]
fn cancellation_after_gate_commit_returns_bounded_and_reaps_tree() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let mut fixture = production_fixture::spawn(&marker).release();
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(fixture.resources.child(), descendant);
    assert_committed_custody(fixture.resources.child());
    let completion = observe_completion(fixture.resources.child_mut());

    let started = Instant::now();
    drop(fixture.resources);

    assert!(started.elapsed() < DROP_RETURN_DEADLINE);
    assert_process_tree_exits(fixture.root, descendant);
    assert_reaped(completion);
    assert_root_is_not_waitable(fixture.root);
}

#[test]
fn closed_fault_receiver_is_settled_on_background_startup_reaper() {
    crate::worker::pty_fault_reaper::FaultReaperReservation::close_next_primary_for_test();
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let mut fixture = production_fixture::spawn(&marker).release();
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(fixture.resources.child(), descendant);
    let completion = observe_completion(fixture.resources.child_mut());

    let started = Instant::now();
    drop(fixture.resources);

    assert!(started.elapsed() < DROP_RETURN_DEADLINE);
    assert_process_tree_exits(fixture.root, descendant);
    assert_reaped(completion);
    assert_root_is_not_waitable(fixture.root);
}

#[tokio::test]
async fn startup_scope_waits_for_final_fault_settlement() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let mut fixture = production_fixture::spawn(&marker).release();
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(fixture.resources.child(), descendant);
    let state = fixture
        .resources
        .child_mut()
        .fault_reaper
        .as_mut()
        .unwrap()
        .observe_state_for_test();

    drop(fixture.resources);
    tokio::time::timeout(DEFERRED_SETTLEMENT_TIMEOUT, fixture.cleanup_scope.settle())
        .await
        .unwrap_or_else(|_| {
            panic!(
                "{SCOPE_SETTLEMENT_TIMEOUT_DETAIL}; latest fault-reaper state: {:#?}",
                state.latest()
            )
        })
        .unwrap();

    assert!(!fixture.root.matches());
    assert!(!descendant.matches());
    assert_root_is_not_waitable(fixture.root);
}

#[test]
fn panic_after_gate_commit_returns_bounded_and_reaps_tree() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let mut fixture = production_fixture::spawn(&marker).release();
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(fixture.resources.child(), descendant);
    assert_committed_custody(fixture.resources.child());
    let completion = observe_completion(fixture.resources.child_mut());

    let started = Instant::now();
    let panic = catch_unwind(AssertUnwindSafe(move || {
        let _resources = fixture.resources;
        panic!("{INJECTED_POST_COMMIT_PANIC}");
    }));

    assert!(panic.is_err());
    assert!(started.elapsed() < DROP_RETURN_DEADLINE);
    assert_process_tree_exits(fixture.root, descendant);
    assert_reaped(completion);
    assert_root_is_not_waitable(fixture.root);
}

#[test]
fn missing_eof_cleanup_keeps_drop_bounded_and_eventually_reaps() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let mut fixture = production_fixture::spawn_signal_resistant(&marker).release();
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(fixture.resources.child(), descendant);
    bind_process_tree_inventory(fixture.resources.child(), fixture.root, descendant);
    let Some(StartupOutputProgress::Exclusive(output)) =
        fixture.resources.child_mut().output_progress.as_mut()
    else {
        panic!("{EXCLUSIVE_OUTPUT_REQUIRED_DETAIL}");
    };
    output.make_quiescent_for_test(Arc::new(AtomicUsize::new(usize::default())));
    let completion = observe_completion(fixture.resources.child_mut());

    let started = Instant::now();
    drop(fixture.resources);

    assert!(started.elapsed() < DROP_RETURN_DEADLINE);
    assert_reaped(completion);
    wait_for_exit(fixture.root);
    wait_for_exit(descendant);
    assert_root_is_not_waitable(fixture.root);
}

#[path = "startup_guard_resource_capture_adversarial_tests.rs"]
mod resource_capture_tests;

fn assert_committed_custody(guard: &StartupChildGuard) {
    assert!(guard.unreleased_gate.is_none());
    assert!(guard.untracked_reaper.is_some());
    assert!(guard.fault_reaper.is_some());
}

fn assert_root_is_not_waitable(root: ProcessIdentity) {
    let mut status = 0;
    let waited = unsafe { libc::waitpid(root.pid as libc::pid_t, &mut status, libc::WNOHANG) };
    assert_eq!(waited, WAITPID_FAILURE);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

fn observe_completion(
    guard: &mut StartupChildGuard,
) -> (
    crate::worker::pty_fault_reaper::FaultReapReceiver,
    crate::worker::pty_fault_reaper::FaultReapStateObserver,
) {
    let reaper = guard.fault_reaper.as_mut().unwrap();
    (
        reaper.observe_startup_completion(),
        reaper.observe_state_for_test(),
    )
}

fn assert_reaped(
    (completion, state): (
        crate::worker::pty_fault_reaper::FaultReapReceiver,
        crate::worker::pty_fault_reaper::FaultReapStateObserver,
    ),
) {
    let outcome = completion
        .receive_timeout(DEFERRED_SETTLEMENT_TIMEOUT)
        .unwrap_or_else(|error| panic!("{error:?}; fault-reaper state: {:#?}", state.latest()));
    assert_eq!(
        outcome.evidence,
        crate::worker::pty_ownership::ReapEvidence::Waited
    );
    assert!(outcome.status.is_some());
}

fn wait_for_exit(identity: ProcessIdentity) {
    let deadline = Instant::now() + DEFERRED_SETTLEMENT_TIMEOUT;
    while identity.matches() {
        assert!(Instant::now() < deadline, "{PROCESS_SURVIVED_DETAIL}");
        std::thread::sleep(MISSING_EOF_POLL_INTERVAL);
    }
}
