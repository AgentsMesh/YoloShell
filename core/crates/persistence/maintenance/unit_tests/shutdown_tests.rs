use super::test_support::{ScannerBarrier, serialize_worker_tests, settle_failsafe};
use super::{MaintenancePlan, MaintenanceWorker};
use std::time::Duration;

const BLOCKING_SCANNER_LABEL: &str = "shutdown-cancellation";
const BLOCKED_SCANNER_SHUTDOWN_GRACE: Duration = Duration::from_millis(500);
const EVENT_TIMEOUT: Duration = Duration::from_secs(2);
const FAILSAFE_TIMEOUT: Duration = Duration::from_secs(5);
const OWNER_FILE: &str = "shutdown-cancellation-owner.lock";
const SHUTDOWN_TIMEOUT_DETAIL: &str = "maintenance worker shutdown timed out";
const UNEXPECTED_SETTLEMENT: &str = "blocked maintenance shutdown settled early";

#[tokio::test]
async fn canceled_async_shutdown_never_falls_back_to_synchronous_join() {
    let _worker_test = serialize_worker_tests().await;
    let root = tempfile::tempdir().unwrap();
    let (mut barrier, scanner) = ScannerBarrier::new(BLOCKING_SCANNER_LABEL);
    let plan = MaintenancePlan::single(root.path().join(OWNER_FILE), vec![Box::new(scanner)]);
    let mut worker = MaintenanceWorker::start(plan);
    tokio::time::timeout(EVENT_TIMEOUT, barrier.wait_entered())
        .await
        .unwrap();
    let failsafe = barrier.arm_failsafe(FAILSAFE_TIMEOUT);

    let mut shutdown = Box::pin(worker.shutdown());
    tokio::select! {
        biased;
        result = &mut shutdown => panic!("{UNEXPECTED_SETTLEMENT}: {result:?}"),
        _ = tokio::task::yield_now() => {}
    }
    drop(shutdown);
    drop(worker);
    assert!(!barrier.was_forced());

    tokio::task::yield_now().await;
    assert!(!barrier.was_forced());
    barrier.release();
    tokio::time::timeout(EVENT_TIMEOUT, barrier.wait_dropped())
        .await
        .unwrap();
    settle_failsafe(failsafe).await.unwrap();
    assert!(!barrier.was_forced());
}

#[tokio::test]
async fn blocked_scanner_is_detached_after_the_shutdown_grace_period() {
    let _worker_test = serialize_worker_tests().await;
    let root = tempfile::tempdir().unwrap();
    let (mut barrier, scanner) = ScannerBarrier::new(BLOCKING_SCANNER_LABEL);
    let plan = MaintenancePlan::single(root.path().join(OWNER_FILE), vec![Box::new(scanner)]);
    let mut worker = MaintenanceWorker::start(plan);
    tokio::time::timeout(EVENT_TIMEOUT, barrier.wait_entered())
        .await
        .unwrap();
    let failsafe = barrier.arm_failsafe(FAILSAFE_TIMEOUT);

    let error = tokio::time::timeout(
        EVENT_TIMEOUT,
        worker.shutdown_with_grace_for_test(BLOCKED_SCANNER_SHUTDOWN_GRACE),
    )
    .await
    .unwrap()
    .unwrap_err();
    assert!(error.to_string().contains(SHUTDOWN_TIMEOUT_DETAIL));
    assert!(!barrier.was_forced());

    barrier.release();
    tokio::time::timeout(EVENT_TIMEOUT, barrier.wait_dropped())
        .await
        .unwrap();
    settle_failsafe(failsafe).await.unwrap();
    assert!(!barrier.was_forced());
}
