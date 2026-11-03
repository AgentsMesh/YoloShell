#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn natural_exit_shuts_down_an_idle_worker_after_the_attachment_grace() {
    let fixture = Fixture::new();
    fixture
        .state
        .runtime_serving
        .store(true, std::sync::atomic::Ordering::Release);
    fixture.state.schedule_natural_exit_shutdown();
    let notified = tokio::time::timeout(
        crate::worker::NATURAL_EXIT_ATTACHMENT_GRACE + SHUTDOWN_GRACE_SLACK,
        fixture.state.shutdown.notified(),
    )
    .await;
    assert!(notified.is_ok());
    assert!(fixture.state.response_shutdown_is_committed());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn finalizing_a_natural_exit_schedules_the_worker_shutdown() {
    let fixture = Fixture::new();
    fixture
        .state
        .runtime_serving
        .store(true, std::sync::atomic::Ordering::Release);
    fixture.state.reader_lifecycle.mark_drained();
    finalize_on_thread(Arc::clone(&fixture.state), Some(0), None, Duration::ZERO).await;
    assert!(fixture.state.termination_reason().is_none());
    let notified = tokio::time::timeout(
        crate::worker::NATURAL_EXIT_ATTACHMENT_GRACE + SHUTDOWN_GRACE_SLACK,
        fixture.state.shutdown.notified(),
    )
    .await;
    assert!(notified.is_ok());
    assert!(fixture.state.response_shutdown_is_committed());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn serving_recovers_a_natural_exit_finalized_before_activation() {
    let fixture = Fixture::new();
    fixture.state.reader_lifecycle.mark_drained();
    finalize_on_thread(Arc::clone(&fixture.state), Some(0), None, Duration::ZERO).await;
    assert!(fixture.state.termination_reason().is_none());
    fixture
        .state
        .runtime_serving
        .store(true, std::sync::atomic::Ordering::Release);
    recover_deferred_natural_exit_shutdown(&fixture.state);
    let notified = tokio::time::timeout(
        crate::worker::NATURAL_EXIT_ATTACHMENT_GRACE + SHUTDOWN_GRACE_SLACK,
        fixture.state.shutdown.notified(),
    )
    .await;
    assert!(notified.is_ok());
    assert!(fixture.state.response_shutdown_is_committed());
}

#[cfg(any(not(test), feature = "yolo-integration-tests"))]
include!("../../integration_tests/pty/missing/pty_missing_eof_finalization_tests.rs");
include!("finalization_namespace_loss_tests.rs");
