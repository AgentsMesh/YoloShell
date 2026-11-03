use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn persistence_failure_after_ready_fences_custody_without_activated() {
    let fixture = Fixture::new();
    let generation = fixture.state.controller_generation.load(Ordering::Acquire);
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    fixture
        .state
        .control_test_hooks
        .fail_next_controller_persist();
    wire.send_ready(&offer).await;

    let mut peer = wire.take_peer();
    tokio::time::timeout(
        ADOPTION_TEST_TIMEOUT,
        acknowledge_exact_worker_release(&mut peer, &offer),
    )
    .await
    .unwrap();
    await_adoption_failure(adoption).await;
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        generation
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
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn indeterminate_controller_write_promotes_and_emits_activated() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    let offer = wire.receive_offer().await;
    wire.receive_commit(&offer).await;
    fixture
        .state
        .control_test_hooks
        .make_next_controller_persist_indeterminate();
    wire.send_ready(&offer).await;

    let mut peer = wire.take_peer();
    tokio::time::timeout(
        ADOPTION_TEST_TIMEOUT,
        receive_exact_worker_activation(&mut peer, &offer, wire.request.controller_generation),
    )
    .await
    .unwrap();
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        wire.request.controller_generation
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        wire.request.controller_generation
    );
    let memory = lock_recover(&fixture.state.record).clone();
    let durable: RuntimeRecord = fixture
        .state
        .namespace
        .read_json(&fixture.state.paths.runtime_record(fixture.terminal_id()))
        .unwrap();
    assert_eq!(durable, memory);
    assert_eq!(
        durable.controller_generation,
        wire.request.controller_generation
    );
    drop(peer);
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn activation_write_failure_retains_promoted_custody_for_retry() {
    let fixture = Fixture::new();
    let predecessor_generation = fixture.state.controller_generation.load(Ordering::Acquire);
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

    drop(wire.take_peer());
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();
    await_adoption_failure(adoption).await;

    assert_promoted_custody_fenced_for_retry(&fixture, &wire, &offer, predecessor_generation).await;
}
