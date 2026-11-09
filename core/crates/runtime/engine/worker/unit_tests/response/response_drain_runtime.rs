use crate::worker::tests::*;
const RESPONSE_DRAIN_RUNTIME_TIMEOUT: Duration = Duration::from_secs(1);

#[tokio::test]
async fn response_drain_pre_abort_barrier_waits_for_reserved_responses() {
    let fixture = Fixture::new();
    let ticket = fixture
        .state
        .reserve_response_publication()
        .unwrap();
    fixture.state.commit_response_shutdown();
    let state = Arc::clone(&fixture.state);
    let mut barrier = tokio::spawn(async move {
        state
            .wait_for_response_drain(RESPONSE_DRAIN_RUNTIME_TIMEOUT)
            .await
    });

    assert!(tokio::time::timeout(Duration::ZERO, &mut barrier).await.is_err());
    ticket.publish();
    assert!(barrier.await.unwrap());
}

#[tokio::test(start_paused = true)]
async fn response_drain_pre_abort_barrier_is_bounded() {
    let fixture = Fixture::new();
    let _ticket = fixture
        .state
        .reserve_response_publication()
        .unwrap();
    fixture.state.commit_response_shutdown();

    assert!(!fixture
        .state
        .wait_for_response_drain(crate::session_timing::WORKER_READY_TIMEOUT)
        .await);
}
