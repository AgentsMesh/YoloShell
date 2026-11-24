use super::*;
use yolo_remote_broker::{BROKER_READY_POLL_INTERVAL, BROKER_READY_TIMEOUT};

#[tokio::test(start_paused = true)]
async fn lock_reconciliation_launch_and_exit_share_one_absolute_deadline() {
    let deadline = ReconcileDeadline::capture(BROKER_READY_TIMEOUT).unwrap();
    let lock_admission = deadline.active();
    let launch = deadline.prepare_launch().unwrap();

    assert_eq!(launch.deadline(), lock_admission);
    assert_eq!(
        tokio::time::Instant::from_std(deadline.exit_delivery()),
        lock_admission
    );
}

#[tokio::test(start_paused = true)]
async fn near_exhausted_lock_budget_is_not_refreshed_for_reconciliation() {
    let deadline = ReconcileDeadline::capture(BROKER_READY_TIMEOUT).unwrap();
    let absolute = deadline.active();
    tokio::time::advance(BROKER_READY_TIMEOUT.saturating_sub(BROKER_READY_POLL_INTERVAL)).await;

    let launch = deadline.prepare_launch().unwrap();

    assert_eq!(launch.deadline(), absolute);
    assert_eq!(
        absolute.duration_since(tokio::time::Instant::now()),
        BROKER_READY_POLL_INTERVAL
    );
}

#[tokio::test(start_paused = true)]
async fn late_first_launch_cannot_extend_its_precomputed_deadline() {
    let deadline = ReconcileDeadline::capture(BROKER_READY_TIMEOUT).unwrap();
    let launch = deadline.prepare_launch().unwrap();
    let absolute = launch.deadline();
    tokio::time::advance(BROKER_READY_TIMEOUT).await;

    let error = launch.verify().unwrap_err();

    assert!(error.to_string().contains(START_DEADLINE_EXPIRED));
    assert_eq!(deadline.active(), absolute);
}

#[tokio::test(start_paused = true)]
async fn expired_start_boundary_prevents_launch_admission() {
    let deadline = ReconcileDeadline::capture(BROKER_READY_TIMEOUT).unwrap();
    tokio::time::advance(BROKER_READY_TIMEOUT).await;

    let error = deadline.prepare_launch().err().unwrap();

    assert!(error.to_string().contains(START_DEADLINE_EXPIRED));
}

#[tokio::test(start_paused = true)]
async fn deadline_overflow_fails_closed() {
    let error = ReconcileDeadline::capture(std::time::Duration::MAX)
        .err()
        .unwrap();

    assert!(error.to_string().contains(START_DEADLINE_OVERFLOW));
}

#[tokio::test(start_paused = true)]
async fn reconciliation_poll_does_not_overshoot_the_absolute_deadline() {
    let deadline = ReconcileDeadline::capture(BROKER_READY_POLL_INTERVAL).unwrap();
    let expected = deadline.active();

    deadline.wait_for_next_attempt(BROKER_READY_TIMEOUT).await;

    assert_eq!(tokio::time::Instant::now(), expected);
}
