use crate::worker::tests::*;
#[cfg(target_os = "macos")]
async fn assert_cancelled_adoption_fenced(
    fixture: &Fixture,
    wire: &mut SuccessorAdoptionWire,
    offer: &crate::pty_custody::CustodyOffer,
    adoption: tokio::task::JoinHandle<Result<()>>,
) {
    let predecessor_generation = lock_recover(&fixture.state.record).controller_generation;
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let mut peer = wire.take_peer();
    adoption.abort();
    let cancellation = tokio::time::timeout(ADOPTION_TEST_TIMEOUT, adoption)
        .await
        .unwrap()
        .unwrap_err();
    assert!(cancellation.is_cancelled());
    let activation = tokio::time::timeout(
        ADOPTION_TEST_TIMEOUT,
        peer.read::<crate::pty_custody::CustodyWorkerReadyDisposition>(),
    )
    .await
    .unwrap();
    assert!(activation.is_err());
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        predecessor_generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        predecessor_generation
    );
    assert!(
        fixture
            .state
            .secondary_custody_authority
            .disconnected_for_test(predecessor)
    );
    assert!(
        fixture
            .state
            .pty_alias_custody
            .release_identity_for_test()
            .is_none()
    );
    let durable: crate::pty_custody::CustodyDurableRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.pty_custody(fixture.terminal_id()))
        .unwrap();
    assert!(durable.matches_candidate(offer, durable.armed_lease().unwrap()));
    assert!(
        fixture
            .state
            .admit_controller_mutation(AuthenticatedController::new(predecessor_generation))
            .await
            .is_err()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn successor_abort_after_durable_rearm_fences_all_custody() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;

    assert_cancelled_adoption_fenced(&fixture, &mut wire, &offer, adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn successor_abort_after_ready_fences_all_custody_without_activated() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .arm();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    wire.send_ready(&offer).await;
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .wait_until_reached()
        .await;

    assert_cancelled_adoption_fenced(&fixture, &mut wire, &offer, adoption).await;
}
