use crate::worker::tests::*;
#[cfg(target_os = "macos")]
const PROMOTION_HOOK_TIMEOUT_DETAIL: &str =
    "successor adoption did not reach the pre-promotion boundary";

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn stalled_authenticated_successor_does_not_hold_global_mutation_gate() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    wire.receive_offer().await;

    let permit = tokio::time::timeout(
        ADOPTION_TEST_SHORT_TIMEOUT,
        fixture.state.admit_system_mutation(),
    )
    .await
    .unwrap();
    drop(permit);
    drop(wire.take_peer());
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn successor_handshake_timeout_preserves_controller_authority() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let generation = fixture.state.controller_generation.load(Ordering::Acquire);
    let persisted = fixture
        .state
        .controller_persisted_generation
        .load(Ordering::Acquire);
    let authority = fixture.state.secondary_custody_authority.active_for_test();
    let release = fixture.state.pty_alias_custody.release_identity_for_test();
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_SHORT_TIMEOUT);
    wire.receive_offer().await;

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
        persisted
    );
    assert_eq!(
        fixture.state.secondary_custody_authority.active_for_test(),
        authority
    );
    assert_eq!(
        fixture.state.pty_alias_custody.release_identity_for_test(),
        release
    );
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn activated_is_not_emitted_before_persistence_and_promotion() {
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
    tokio::time::timeout(
        ADOPTION_TEST_TIMEOUT,
        fixture
            .state
            .control_test_hooks
            .custody_before_promotion
            .wait_until_reached(),
    )
    .await
    .expect(PROMOTION_HOOK_TIMEOUT_DETAIL);
    let mut peer = wire.take_peer();
    let activation_offer = offer.clone();
    let activation_generation = wire.request.controller_generation;
    let activation = tokio::spawn(async move {
        receive_exact_worker_activation(&mut peer, &activation_offer, activation_generation).await;
        peer
    });
    assert_task_pending(&activation).await;
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

    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();
    let peer = tokio::time::timeout(ADOPTION_TEST_TIMEOUT, activation)
        .await
        .unwrap()
        .unwrap();
    let identity = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    assert_eq!(identity.lease_id(), offer.lease_id);
    assert_eq!(identity.offer_id(), offer.offer_id);
    assert_eq!(identity.generation(), wire.request.controller_generation);
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        wire.request.controller_generation
    );
    assert_eq!(
        fixture.state.pty_alias_custody.release_identity_for_test(),
        Some(identity)
    );
    drop(peer);
    await_adoption_failure(adoption).await;
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn same_value_custody_replacement_fences_runtime_promotion() {
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
    let path = fixture.state.paths.pty_custody(fixture.terminal_id());
    let durable: crate::pty_custody::CustodyDurableRecord =
        fixture.state.namespace.read_json(&path).unwrap();
    fixture.state.namespace.write_json(&path, &durable).unwrap();
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();

    let mut peer = wire.take_peer();
    acknowledge_exact_worker_release(&mut peer, &offer).await;
    await_adoption_failure(adoption).await;
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
}
