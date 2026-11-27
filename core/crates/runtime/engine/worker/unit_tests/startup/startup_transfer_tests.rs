use super::process_assertions::{
    assert_process_exits, assert_process_tree_exits, bind_process_tree_inventory,
};
use super::tests::{
    TEST_MARKER_FILE, production_fixture, spawn_test_guard, wait_for_descendant,
    wait_for_tracked_identity,
};
use crate::worker::startup::startup_scope::StartupScopeMonitor;
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

const INJECTED_MONITOR_FAILURE: &str = "injected startup monitor refresh failure";
const INJECTED_SIGNAL_FAILURE: &str = "injected startup exact-signal failure";
const CLEANUP_RETURN_PROBE: Duration = Duration::from_millis(50);
const CLEANUP_SETTLEMENT_TIMEOUT: Duration = super::super::WORKER_TERMINATION_TIMEOUT;
const TEST_SLEEP_EXECUTABLE: &str = "/bin/sleep";
const TEST_SLEEP_DURATION: &str = "30";
const RENDEZVOUS_CHANNEL_CAPACITY: usize = 0;
const FIRST_EXACT_SIGNAL_TARGET_INDEX: usize = 0;
const FAULT_REAP_STATUS_MISSING_DETAIL: &str = "fault-reap status was not published";
const FAULT_REAP_SNAPSHOT_MISSING_DETAIL: &str = "fault-reap state snapshot was not published";

#[test]
fn transferred_ownership_remains_armed_until_the_owner_settles_it() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let (mut guard, _master, root) = spawn_test_guard(&marker);
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(&guard, descendant);
    guard.install_reader_backed_output_for_test();
    let ownership = guard.take_ownership().unwrap();
    guard.disarm_after_transfer();

    drop(ownership);

    assert_process_tree_exits(root, descendant);
}

#[test]
fn monitor_failure_blocks_transfer_but_guard_still_reaps_the_tree() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let (mut guard, _master, root) = spawn_test_guard(&marker);
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(&guard, descendant);
    guard.monitor = Some(StartupScopeMonitor::failed_for_test(
        INJECTED_MONITOR_FAILURE,
    ));

    let refresh_error = guard.refresh().unwrap_err();
    assert!(refresh_error.to_string().contains(INJECTED_MONITOR_FAILURE));
    let transfer_error = guard.take_ownership().err().unwrap();
    assert!(
        transfer_error
            .to_string()
            .contains(INJECTED_MONITOR_FAILURE)
    );
    drop(guard);

    assert_process_tree_exits(root, descendant);
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn exact_signal_failure_withholds_startup_return_without_blocking_wait() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let production_fixture::HeldStartup {
        mut resources,
        root,
        release,
        cleanup_scope,
    } = production_fixture::spawn(&marker);
    release.release();
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(resources.child(), descendant);
    bind_process_tree_inventory(resources.child(), root, descendant);
    let state = resources
        .child_mut()
        .fault_reaper
        .as_mut()
        .unwrap()
        .observe_state_for_test();
    let (first_signal_attempted, first_signal_observed) =
        std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let (first_signal_release, await_first_signal_release) =
        std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let (second_signal_attempted, second_signal_observed) =
        std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let (second_signal_release, await_second_signal_release) =
        std::sync::mpsc::sync_channel(RENDEZVOUS_CHANNEL_CAPACITY);
    let (sender, receiver) = std::sync::mpsc::channel();
    let cleanup = std::thread::spawn(move || {
        let _hook = crate::process::install_test_signal_hook(
            libc::SIGKILL,
            FIRST_EXACT_SIGNAL_TARGET_INDEX,
            move || {
                first_signal_attempted.send(()).unwrap();
                await_first_signal_release.recv().unwrap();
            },
        )
        .then(move || {
            second_signal_attempted.send(()).unwrap();
            await_second_signal_release.recv().unwrap();
        });
        let _failure = crate::process::install_test_signal_failure(
            libc::SIGKILL,
            FIRST_EXACT_SIGNAL_TARGET_INDEX,
            INJECTED_SIGNAL_FAILURE,
        );
        resources.close_owner_master().unwrap();
        let result = resources.child_mut().terminate_with_owned_output();
        assert!(!resources.child().requires_cleanup());
        sender.send(result).unwrap();
    });

    first_signal_observed
        .recv_timeout(CLEANUP_SETTLEMENT_TIMEOUT)
        .unwrap();
    assert!(matches!(
        receiver.recv_timeout(CLEANUP_RETURN_PROBE),
        Err(RecvTimeoutError::Timeout)
    ));
    first_signal_release.send(()).unwrap();
    second_signal_observed
        .recv_timeout(CLEANUP_SETTLEMENT_TIMEOUT)
        .unwrap();
    assert!(matches!(
        receiver.recv_timeout(CLEANUP_RETURN_PROBE),
        Err(RecvTimeoutError::Timeout)
    ));
    let status = state.status().expect(FAULT_REAP_STATUS_MISSING_DETAIL);
    assert!(status.custody_retained);
    assert!(
        status
            .last_fault
            .as_deref()
            .is_some_and(|fault| fault.contains(INJECTED_SIGNAL_FAILURE))
    );
    let snapshot = state.latest().expect(FAULT_REAP_SNAPSHOT_MISSING_DETAIL);
    assert!(snapshot.custody.is_some());
    assert!(snapshot.failure_present);
    assert!(!snapshot.completion_published);
    second_signal_release.send(()).unwrap();
    let error = receiver
        .recv_timeout(CLEANUP_SETTLEMENT_TIMEOUT)
        .unwrap()
        .unwrap_err();
    assert!(format!("{error:#}").contains(INJECTED_SIGNAL_FAILURE));
    cleanup.join().unwrap();
    cleanup_scope.settle().await.unwrap();
    assert_process_tree_exits(root, descendant);
}

#[test]
fn identity_binding_failure_retains_unmoved_child_in_guard() {
    let child = crate::process::spawn_waitable_test_child(
        std::process::Command::new(TEST_SLEEP_EXECUTABLE).arg(TEST_SLEEP_DURATION),
    );
    let root = crate::process::ProcessIdentity::capture(child.id()).unwrap();
    let mut guard = super::StartupChildGuard::new(Box::new(child));
    let failure = crate::process::install_test_match_failure(INJECTED_MONITOR_FAILURE);

    assert!(guard.track(root).is_err());
    assert!(guard.child.is_some());
    assert!(guard.authority.is_none());
    drop(failure);
    drop(guard);
    assert_process_exits(root);
}

#[test]
fn active_tree_observer_cannot_consume_authority_before_transfer() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(TEST_MARKER_FILE);
    let (mut guard, _master, root) = spawn_test_guard(&marker);
    let descendant = wait_for_descendant(&marker);
    wait_for_tracked_identity(&guard, descendant);
    guard.stop_monitor().unwrap();
    let observer = Arc::clone(guard.tree.as_ref().unwrap());

    assert!(guard.take_stopped_authority().is_err());
    assert!(guard.child.is_none());
    assert!(guard.authority.is_some());
    assert!(guard.tree.is_some());
    drop(observer);
    drop(guard);
    assert_process_tree_exits(root, descendant);
}
