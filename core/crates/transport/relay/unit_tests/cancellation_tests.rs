use super::*;
use std::time::Duration;

const WAIT_TIMEOUT: Duration = Duration::from_secs(1);
const WAITER_COUNT: usize = 8;

#[tokio::test]
async fn cancellation_is_idempotent_visible_to_clones_and_fail_closed() {
    let cancellation = RelayCancellation::default();
    let observer = cancellation.clone();
    assert!(cancellation.check().is_ok());
    cancellation.cancel();
    cancellation.cancel();
    tokio::time::timeout(WAIT_TIMEOUT, observer.cancelled())
        .await
        .unwrap();
    assert!(observer.is_cancelled());
    assert!(matches!(observer.check(), Err(RelayError::Cancelled)));
}

#[tokio::test]
async fn wait_returns_ready_output_and_interrupts_pending_work() {
    let active = RelayCancellation::default();
    assert_eq!(active.wait(async { 7_u8 }).await.unwrap(), 7);

    let cancelled = RelayCancellation::default();
    cancelled.cancel();
    let result = cancelled.wait(std::future::pending::<()>()).await;
    assert!(matches!(result, Err(RelayError::Cancelled)));
}

#[tokio::test]
async fn cancellation_wakes_every_registered_waiter() {
    let cancellation = RelayCancellation::default();
    let tasks: Vec<_> = (0..WAITER_COUNT)
        .map(|_| {
            let observer = cancellation.clone();
            tokio::spawn(async move { observer.cancelled().await })
        })
        .collect();
    tokio::task::yield_now().await;
    cancellation.cancel();
    for task in tasks {
        tokio::time::timeout(WAIT_TIMEOUT, task)
            .await
            .unwrap()
            .unwrap();
    }
}
