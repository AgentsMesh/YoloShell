use super::super::{
    cleanup_lease::{self, LeaseState},
    lease,
};
use super::*;
use crate::paths::constants::SOCKET_STAGE_LEASE_TIMEOUT;
use crate::paths::identity::ObjectIdentity;
use crate::paths::process_file_lock::ProcessLockTurn;
use std::time::{Duration, Instant};

const ALTERNATE_ENDPOINT_NAME: &str = "alternate.sock";
const EXPECTED_WAITER_COUNT: usize = 1;
const LIFECYCLE_RELEASE_MARGIN: Duration = Duration::from_millis(100);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const TEST_TIMEOUT: Duration = Duration::from_secs(3);
const CLEANUP_ACQUISITION_ERROR: &str = "cleanup lease was not acquired";
const CANCELLED_OWNERSHIP_ERROR: &str = "cancelled lifecycle waiter retained lock ownership";
const WAITER_QUEUE_ERROR: &str = "lifecycle waiter was not queued";
const UNEXPECTED_TARGET_ACQUISITION: &str = "held target lease was acquired";

#[tokio::test(flavor = "current_thread")]
async fn lifecycle_contention_can_outlive_the_target_lease_budget() {
    let (_temporary, directory, target) = super::fixture();
    let lifecycle = lease::acquire_lifecycle(&directory, &target.lifecycle_path())
        .unwrap()
        .unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(SOCKET_STAGE_LEASE_TIMEOUT + LIFECYCLE_RELEASE_MARGIN);
        drop(lifecycle);
    });
    let started = Instant::now();

    let acquired = Lease::acquire(&directory, &target).await.unwrap();

    assert!(started.elapsed() >= SOCKET_STAGE_LEASE_TIMEOUT);
    drop(acquired);
    release.join().unwrap();
}

#[test]
fn cleanup_target_lease_does_not_convoy_an_unrelated_target() {
    let (_temporary, directory, target) = super::fixture();
    drop(Lease::acquire_immediate_for_test(&directory, &target).unwrap());
    let cleanup =
        match cleanup_lease::try_cleanup(&directory, target.key(), directory.path()).unwrap() {
            LeaseState::Acquired(cleanup) => cleanup,
            _ => panic!("{CLEANUP_ACQUISITION_ERROR}"),
        };
    let alternate_name = CString::new(ALTERNATE_ENDPOINT_NAME).unwrap();
    let alternate = Target::new(directory.path(), &alternate_name).unwrap();

    let unrelated = Lease::acquire_immediate_for_test(&directory, &alternate).unwrap();
    let target_error = match Lease::acquire_immediate_for_test(&directory, &target) {
        Ok(_) => panic!("{UNEXPECTED_TARGET_ACQUISITION}"),
        Err(error) => error,
    };

    assert_eq!(target_error.to_string(), lease::TARGET_BUSY_ERROR);
    drop(unrelated);
    drop(cleanup);
}

#[tokio::test(flavor = "current_thread")]
async fn cancelling_a_lifecycle_wait_does_not_leak_ownership() {
    let (_temporary, directory, target) = super::fixture();
    let lifecycle = lease::acquire_lifecycle(&directory, &target.lifecycle_path())
        .unwrap()
        .unwrap();
    let identity =
        ObjectIdentity::from_file(&std::fs::File::open(target.lifecycle_path()).unwrap())
            .unwrap()
            .unwrap();
    let waiting_directory = directory.try_clone().unwrap();
    let waiting_target = target.clone();
    let waiter =
        tokio::spawn(async move { Lease::acquire(&waiting_directory, &waiting_target).await });
    wait_for_queued_waiter(identity).await;

    waiter.abort();
    assert!(matches!(waiter.await, Err(error) if error.is_cancelled()));
    drop(lifecycle);
    wait_for_immediate_acquisition(&directory, &target).await;
}

async fn wait_for_queued_waiter(identity: ObjectIdentity) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    while ProcessLockTurn::queued_waiters_for_test(identity) != EXPECTED_WAITER_COUNT {
        assert!(Instant::now() < deadline, "{WAITER_QUEUE_ERROR}");
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn wait_for_immediate_acquisition(directory: &PrivateDirectory, target: &Target) {
    let deadline = Instant::now() + TEST_TIMEOUT;
    loop {
        if let Ok(lease) = Lease::acquire_immediate_for_test(directory, target) {
            drop(lease);
            return;
        }
        assert!(Instant::now() < deadline, "{CANCELLED_OWNERSHIP_ERROR}");
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}
