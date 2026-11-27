use super::{
    try_waitable_child_spawn_guard, waitable_child_spawn_guard, waitable_child_spawn_guard_before,
};
use std::sync::mpsc::{RecvTimeoutError, channel};
use std::time::{Duration, Instant};

const LOCK_PROBE_TIMEOUT: Duration = Duration::from_millis(50);
const LOCK_RELEASE_TIMEOUT: Duration = Duration::from_secs(1);
const BOUNDARY_PANIC: &str = "injected child spawn boundary panic";
const CLEANUP_ACQUIRE_FAILURE: &str =
    "cleanup spawn boundary stayed busy after active boundary release";
const SCAN_ACQUIRE_FAILURE: &str = "descriptor scan gate stayed busy after spawn release";
const NORMALIZATION_FAILURE_MISSING: &str =
    "cleanup spawn boundary unexpectedly normalized SIGCHLD";

#[test]
fn child_spawn_boundaries_are_process_serialized() {
    let first = waitable_child_spawn_guard().unwrap();
    let (attempting_sender, attempting_receiver) = channel();
    let (acquired_sender, acquired_receiver) = channel();
    let contender = std::thread::spawn(move || {
        attempting_sender.send(()).unwrap();
        let _second = waitable_child_spawn_guard().unwrap();
        acquired_sender.send(()).unwrap();
    });
    attempting_receiver.recv().unwrap();

    assert_eq!(
        acquired_receiver.recv_timeout(LOCK_PROBE_TIMEOUT),
        Err(RecvTimeoutError::Timeout)
    );
    drop(first);
    acquired_receiver
        .recv_timeout(LOCK_RELEASE_TIMEOUT)
        .unwrap();
    contender.join().unwrap();
}

#[test]
fn descriptor_scan_boundary_defers_until_the_active_boundary_releases() {
    let first = waitable_child_spawn_guard().unwrap();
    assert!(super::try_descriptor_scan_guard().is_none());
    drop(first);
    let _scan = acquire_descriptor_scan_guard();
}

#[test]
fn cleanup_spawn_boundary_never_waits_behind_an_active_boundary() {
    let first = waitable_child_spawn_guard().unwrap();
    assert!(try_waitable_child_spawn_guard().unwrap().is_none());
    drop(first);
    let _cleanup = acquire_cleanup_spawn_guard();
}

#[test]
fn cleanup_spawn_boundary_releases_gate_after_normalization_failure() {
    let failure = super::test_hook::fail_sigchld_normalization();
    let deadline = Instant::now() + LOCK_RELEASE_TIMEOUT;
    loop {
        match try_waitable_child_spawn_guard() {
            Err(error) => {
                assert_eq!(
                    error.to_string(),
                    super::test_hook::NORMALIZATION_FAILURE_DETAIL
                );
                break;
            }
            Ok(None) => {
                assert!(Instant::now() < deadline, "{NORMALIZATION_FAILURE_MISSING}");
                std::thread::yield_now();
            }
            Ok(Some(_)) => panic!("{NORMALIZATION_FAILURE_MISSING}"),
        }
    }
    drop(failure);

    let _next = waitable_child_spawn_guard_before(Instant::now() + LOCK_RELEASE_TIMEOUT).unwrap();
}

#[test]
fn waiting_child_spawn_has_priority_over_descriptor_scans() {
    let waiter = super::reserve_spawn_waiter_for_test();
    assert!(super::try_descriptor_scan_guard().is_none());
    assert!(try_waitable_child_spawn_guard().unwrap().is_none());
    drop(waiter);
    let _scan = acquire_descriptor_scan_guard();
}

#[test]
fn descriptor_scan_guard_does_not_normalize_sigchld() {
    super::clear_normalization_observation();
    let spawn = waitable_child_spawn_guard().unwrap();
    assert!(super::normalization_was_observed());
    drop(spawn);

    super::clear_normalization_observation();
    let _scan = acquire_descriptor_scan_guard();
    assert!(!super::normalization_was_observed());
}

#[test]
fn child_spawn_boundary_is_released_during_unwind() {
    let panic = std::panic::catch_unwind(|| {
        let _guard = waitable_child_spawn_guard().unwrap();
        panic!("{BOUNDARY_PANIC}");
    });

    assert!(panic.is_err());
    let _next = waitable_child_spawn_guard().unwrap();
}

fn acquire_descriptor_scan_guard() -> super::DescriptorScanGuard {
    let deadline = Instant::now() + LOCK_RELEASE_TIMEOUT;
    loop {
        if let Some(guard) = super::try_descriptor_scan_guard() {
            return guard;
        }
        assert!(Instant::now() < deadline, "{SCAN_ACQUIRE_FAILURE}");
        std::thread::yield_now();
    }
}

fn acquire_cleanup_spawn_guard() -> super::WaitableChildSpawnGuard {
    let deadline = Instant::now() + LOCK_RELEASE_TIMEOUT;
    loop {
        if let Some(guard) = try_waitable_child_spawn_guard().unwrap() {
            return guard;
        }
        assert!(Instant::now() < deadline, "{CLEANUP_ACQUIRE_FAILURE}");
        std::thread::yield_now();
    }
}
