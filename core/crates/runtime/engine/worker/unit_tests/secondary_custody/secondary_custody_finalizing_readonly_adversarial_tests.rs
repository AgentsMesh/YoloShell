use crate::worker::tests::*;
#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn pending_termination_fences_read_only_successor_promotion() {
    let fixture = Fixture::new();
    mark_finalizing(&fixture);
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .arm();
    let (mut wire, offer, adoption) = adopt_finalizing(&fixture).await;
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .wait_until_reached()
        .await;

    let permit = fixture
        .state
        .admit_controller_mutation(fixture.controller())
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&permit, ExitReason::UserTerminate);
    drop(permit);
    let predecessor_generation = fixture.state.controller_generation.load(Ordering::Acquire);
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
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn child_wait_publication_serializes_before_running_successor_promotion() {
    let fixture = Fixture::new();
    let predecessor_generation = fixture.state.controller_generation.load(Ordering::Acquire);
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .arm();
    fixture
        .state
        .reader_lifecycle
        .child_wait_publication_after_lock
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

    let state = Arc::clone(&fixture.state);
    let publication = std::thread::spawn(move || state.publish_direct_child_waited());
    fixture
        .state
        .reader_lifecycle
        .child_wait_publication_after_lock
        .wait_until_reached();
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();
    fixture
        .state
        .reader_lifecycle
        .child_wait_publication_after_lock
        .resume();
    publication.join().unwrap();

    let mut peer = wire.take_peer();
    acknowledge_exact_worker_release(&mut peer, &offer).await;
    await_adoption_failure(adoption).await;
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        predecessor_generation
    );
}
