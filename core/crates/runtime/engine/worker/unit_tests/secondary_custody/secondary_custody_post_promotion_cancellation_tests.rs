use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn cancellation_after_promotion_retains_promoted_custody_for_retry() {
    let fixture = Fixture::new();
    let predecessor_generation = fixture.state.controller_generation.load(Ordering::Acquire);
    fixture
        .state
        .control_test_hooks
        .custody_before_activation
        .arm();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    fixture
        .state
        .control_test_hooks
        .custody_before_activation
        .wait_until_reached()
        .await;

    adoption.abort();
    let cancellation = tokio::time::timeout(ADOPTION_TEST_TIMEOUT, adoption)
        .await
        .unwrap()
        .unwrap_err();
    assert!(cancellation.is_cancelled());

    assert_promoted_custody_fenced_for_retry(&fixture, &wire, &offer, predecessor_generation).await;
}
