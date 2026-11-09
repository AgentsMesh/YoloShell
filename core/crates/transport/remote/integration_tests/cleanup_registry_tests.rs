use super::*;
use crate::remote::constants::{
    CLEANUP_REGISTRY_CLOSED_MESSAGE, CLEANUP_REGISTRY_FAILED_MESSAGE,
    CLEANUP_REGISTRY_TIMEOUT_MESSAGE,
};

const TEST_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(1);

#[tokio::test]
async fn settlement_waits_for_registered_cleanup_and_seals_admission() {
    let registry = RemoteCleanupRegistry::default();
    let reservation = registry.reserve().unwrap();
    let (release, released) = tokio::sync::oneshot::channel();
    reservation.launch(async move {
        let _ = released.await;
        Ok(())
    });
    let settling = tokio::spawn({
        let registry = registry.clone();
        async move { registry.settle(TEST_SETTLEMENT_TIMEOUT).await }
    });
    tokio::task::yield_now().await;
    let error = registry.reserve().err().unwrap();
    assert!(error.to_string().contains(CLEANUP_REGISTRY_CLOSED_MESSAGE));
    assert!(!settling.is_finished());
    release.send(()).unwrap();
    settling.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn settlement_retains_a_notification_before_its_wait_is_polled() {
    let registry = RemoteCleanupRegistry::default();
    let reservation = registry.reserve().unwrap();
    let barrier = registry.block_next_wait_for_test();
    let settling = tokio::spawn({
        let registry = registry.clone();
        async move { registry.settle(TEST_SETTLEMENT_TIMEOUT).await }
    });
    barrier.wait_entered().await;

    drop(reservation);
    barrier.release();

    settling.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn settlement_deadline_retains_blocked_cleanup_custody() {
    let registry = RemoteCleanupRegistry::default();
    let (release, released) = tokio::sync::oneshot::channel();
    let (completed, completion) = tokio::sync::oneshot::channel();
    registry.reserve().unwrap().launch(async move {
        let _ = released.await;
        let _ = completed.send(());
        Ok(())
    });
    let settling = tokio::spawn({
        let registry = registry.clone();
        async move { registry.settle(TEST_SETTLEMENT_TIMEOUT).await }
    });
    tokio::task::yield_now().await;
    assert!(!settling.is_finished());
    tokio::time::advance(TEST_SETTLEMENT_TIMEOUT).await;
    let error = settling.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(CLEANUP_REGISTRY_TIMEOUT_MESSAGE));
    release.send(()).unwrap();
    completion.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn late_cleanup_runs_after_a_live_reservation_forces_fail_stop() {
    let registry = RemoteCleanupRegistry::default();
    let reservation = registry.reserve().unwrap();
    let settling = tokio::spawn({
        let registry = registry.clone();
        async move { registry.settle(TEST_SETTLEMENT_TIMEOUT).await }
    });
    tokio::task::yield_now().await;
    tokio::time::advance(TEST_SETTLEMENT_TIMEOUT).await;
    let error = settling.await.unwrap().unwrap_err();
    assert!(error.to_string().contains(CLEANUP_REGISTRY_TIMEOUT_MESSAGE));

    let (executed, observed) = tokio::sync::oneshot::channel();
    reservation.launch(async move {
        let _ = executed.send(());
        Ok(())
    });
    observed.await.unwrap();

    assert!(registry.settle(TEST_SETTLEMENT_TIMEOUT).await.is_err());
}

#[tokio::test]
async fn failed_cleanup_is_retained_as_a_repeatable_terminal_result() {
    let registry = RemoteCleanupRegistry::default();
    registry
        .reserve()
        .unwrap()
        .launch(async move { Err(RemoteError::transport(CLEANUP_REGISTRY_FAILED_MESSAGE)) });
    assert!(registry.settle(TEST_SETTLEMENT_TIMEOUT).await.is_err());
    assert!(registry.settle(TEST_SETTLEMENT_TIMEOUT).await.is_err());
}

#[tokio::test]
async fn cancelled_settlement_returns_join_handle_custody_for_retry() {
    let registry = RemoteCleanupRegistry::default();
    let (entered, observed) = tokio::sync::oneshot::channel();
    let (release, released) = tokio::sync::oneshot::channel();
    registry.reserve().unwrap().launch(async move {
        let _ = entered.send(());
        let _ = released.await;
        Ok(())
    });
    let settling = tokio::spawn({
        let registry = registry.clone();
        async move { registry.settle(TEST_SETTLEMENT_TIMEOUT).await }
    });
    observed.await.unwrap();
    tokio::task::yield_now().await;

    settling.abort();
    assert!(settling.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    registry.settle(TEST_SETTLEMENT_TIMEOUT).await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn settlement_timeout_retains_entire_cleanup_batch() {
    let registry = RemoteCleanupRegistry::default();
    let (first_release, first_entered, first_completed) = observable_cleanup(&registry);
    let (second_release, second_entered, second_completed) = observable_cleanup(&registry);
    first_entered.await.unwrap();
    second_entered.await.unwrap();
    let settling = tokio::spawn({
        let registry = registry.clone();
        async move { registry.settle(TEST_SETTLEMENT_TIMEOUT).await }
    });

    tokio::task::yield_now().await;
    tokio::time::advance(TEST_SETTLEMENT_TIMEOUT).await;
    assert!(settling.await.unwrap().is_err());

    first_release.send(()).unwrap();
    second_release.send(()).unwrap();
    first_completed.await.unwrap();
    second_completed.await.unwrap();
}

#[tokio::test]
async fn registry_drop_detaches_without_cancelling_cleanup() {
    let registry = RemoteCleanupRegistry::default();
    let (release, entered, completed) = observable_cleanup(&registry);
    entered.await.unwrap();

    drop(registry);
    release.send(()).unwrap();
    completed.await.unwrap();
}

fn observable_cleanup(
    registry: &RemoteCleanupRegistry,
) -> (
    tokio::sync::oneshot::Sender<()>,
    tokio::sync::oneshot::Receiver<()>,
    tokio::sync::oneshot::Receiver<()>,
) {
    let (release, released) = tokio::sync::oneshot::channel();
    let (entered, observed) = tokio::sync::oneshot::channel();
    let (completed, completion) = tokio::sync::oneshot::channel();
    registry.reserve().unwrap().launch(async move {
        let _ = entered.send(());
        let _ = released.await;
        let _ = completed.send(());
        Ok(())
    });
    (release, observed, completion)
}
