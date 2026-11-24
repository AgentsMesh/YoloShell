use super::*;
use crate::pending::PendingObservation;
use crate::pending_fixture::FakePending;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use yolo_remote_broker::{
    BROKER_PROTOCOL_VERSION, BROKER_READY_POLL_INTERVAL, BROKER_SPAWN_RETRY_INTERVAL, BrokerStatus,
};

const EXPECTED_ATTEMPTS: usize = 1;
const PRIMARY_FAILURE: &str = "primary reconcile failure";
const CLEANUP_FAILURE: &str = "pending launch cleanup failure";
const DUPLICATE_ADMISSION_FAILURE: &str = "duplicate launch admission";
const EARLY_TASK_EXIT: &str = "tracked broker task failed";
const OBSERVATION_FAILURE: &str = "pending launch observation failure";

#[tokio::test(start_paused = true)]
async fn successful_unobservable_launch_retains_exclusive_admission() {
    let mut authority = SpawnAuthority::new(BROKER_SPAWN_RETRY_INTERVAL);
    assert!(authority.can_admit(tokio::time::Instant::now()));

    authority.begin().unwrap();
    authority.finish(Ok(FakePending::unmanaged())).unwrap();
    tokio::time::advance(BROKER_SPAWN_RETRY_INTERVAL).await;
    tokio::time::advance(BROKER_READY_POLL_INTERVAL).await;
    authority.refresh().unwrap();

    assert!(!authority.can_admit(tokio::time::Instant::now()));
    assert_eq!(authority.attempts(), EXPECTED_ATTEMPTS);
    authority.commit(&test_status()).unwrap();
}

fn test_status() -> BrokerStatus {
    BrokerStatus {
        protocol_version: BROKER_PROTOCOL_VERSION,
        adopted: true,
        instance_id: uuid::Uuid::new_v4(),
        host_id: crate::fixture_constants::TEST_HOST_ID.into(),
        connection_id: None,
        connection_generation: u64::default(),
        config_revision: crate::fixture_constants::TEST_CONFIG_REVISION.into(),
        capabilities: Vec::new(),
    }
}

#[test]
fn failed_reconcile_explicitly_compensates_and_combines_cleanup_failure() {
    let compensated = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&compensated);
    let mut authority = SpawnAuthority::new(BROKER_SPAWN_RETRY_INTERVAL);
    authority.begin().unwrap();
    authority
        .finish(Ok(FakePending::tracked(move || {
            observed.store(true, Ordering::SeqCst);
            anyhow::bail!(CLEANUP_FAILURE)
        })))
        .unwrap();

    let error = authority
        .resolve::<()>(Err(anyhow::anyhow!(PRIMARY_FAILURE)))
        .unwrap_err();

    assert!(compensated.load(Ordering::SeqCst));
    assert!(error.to_string().contains(PRIMARY_FAILURE));
    assert!(error.to_string().contains(CLEANUP_FAILURE));
}

#[test]
fn spawn_attempt_overflow_fails_before_admission() {
    let mut authority = SpawnAuthority::new(BROKER_SPAWN_RETRY_INTERVAL);
    authority.attempts = usize::MAX;

    let error = authority.begin().unwrap_err();

    assert!(error.to_string().contains(ATTEMPT_OVERFLOW_DETAIL));
}

#[test]
fn observation_failure_explicitly_compensates_and_combines_both_errors() {
    let compensated = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&compensated);
    let mut authority = SpawnAuthority::new(BROKER_SPAWN_RETRY_INTERVAL);
    authority.begin().unwrap();
    authority
        .finish(Ok(FakePending::tracked_pending(
            || anyhow::bail!(OBSERVATION_FAILURE),
            move || {
                observed.store(true, Ordering::SeqCst);
                anyhow::bail!(CLEANUP_FAILURE)
            },
        )))
        .unwrap();

    let error = authority.refresh().unwrap_err();

    assert!(compensated.load(Ordering::SeqCst));
    assert!(error.to_string().contains(OBSERVATION_FAILURE));
    assert!(error.to_string().contains(CLEANUP_FAILURE));
    authority.resolve(Ok(())).unwrap();
}

#[tokio::test]
async fn later_spawn_failure_does_not_overwrite_early_exit_root_cause() {
    let mut authority = SpawnAuthority::new(BROKER_SPAWN_RETRY_INTERVAL);
    authority.begin().unwrap();
    authority
        .finish(Ok(FakePending::tracked_pending(
            || Ok(PendingObservation::Exited(EARLY_TASK_EXIT.to_owned())),
            || Ok(()),
        )))
        .unwrap();

    authority.refresh().unwrap();
    tokio::time::sleep(BROKER_SPAWN_RETRY_INTERVAL).await;
    authority.begin().unwrap();
    authority
        .finish(Err(anyhow::anyhow!(DUPLICATE_ADMISSION_FAILURE)))
        .unwrap();

    assert!(
        authority
            .last_exit_error()
            .unwrap()
            .contains(EARLY_TASK_EXIT)
    );
    assert_eq!(
        authority.last_spawn_error(),
        Some(DUPLICATE_ADMISSION_FAILURE)
    );
    authority.resolve(Ok(())).unwrap();
}
