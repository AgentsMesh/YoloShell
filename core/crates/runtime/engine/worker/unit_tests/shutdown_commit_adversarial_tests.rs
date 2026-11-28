use crate::worker::tests::*;
const SHUTDOWN_COMMIT_PROMOTION_DETAIL: &str =
    "shutdown-committed worker accepted a successor custody promotion";

#[tokio::test]
async fn committed_shutdown_fences_direct_successor_promotion() {
    let fixture = Fixture::new();
    let predecessor = fixture
        .state
        .secondary_custody_authority
        .active_for_test()
        .unwrap();
    let successor = secondary_custody_authority::ReadyWorkerCustodyIdentity::fresh_for_test(
        CUSTODY_SUCCESSOR_GENERATION,
    );
    let (successor_lease, _successor_peer) = test_custody_lease(&fixture, successor);
    let permit = fixture.state.admit_system_mutation().await;

    fixture.state.commit_system_shutdown(&permit);
    let promotion = fixture
        .state
        .promote_ready_custody_for_test(&permit, &successor_lease);

    assert_eq!(
        promotion.unwrap_err().message,
        secondary_custody_promotion::SUCCESSOR_SHUTDOWN_COMMITTED_DETAIL,
        "{SHUTDOWN_COMMIT_PROMOTION_DETAIL}"
    );
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        FIXTURE_CONTROLLER_GENERATION
    );
    assert_eq!(
        fixture.state.secondary_custody_authority.active_for_test(),
        Some(predecessor)
    );
}

#[tokio::test(start_paused = true)]
async fn delayed_controller_shutdown_commits_latch() {
    let fixture = Fixture::new();
    let notified = fixture.state.shutdown.notified();
    tokio::pin!(notified);

    fixture
        .state
        .schedule_controller_shutdown(fixture.controller());
    tokio::task::yield_now().await;
    tokio::time::advance(EXPLICIT_EXIT_SHUTDOWN_DELAY).await;
    tokio::task::yield_now().await;
    tokio::time::timeout(Duration::ZERO, &mut notified)
        .await
        .unwrap();

    assert!(fixture.state.response_shutdown_is_committed());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn finalizer_enqueued_shutdown_fences_successor_before_activation() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .arm();
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

    let controller = fixture.controller();
    let permit = fixture
        .state
        .admit_controller_mutation(controller)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&permit, ExitReason::UserTerminate);
    drop(permit);
    let shutdown = fixture.state.shutdown.notified();
    tokio::pin!(shutdown);
    let state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || state.notify_termination_shutdown_blocking())
        .await
        .unwrap();
    let mut peer = wire.take_peer();
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();

    tokio::time::timeout(
        ADOPTION_TEST_TIMEOUT,
        acknowledge_exact_worker_release(&mut peer, &offer),
    )
    .await
    .unwrap();
    await_adoption_failure(adoption).await;
    tokio::time::timeout(ADOPTION_TEST_TIMEOUT, &mut shutdown)
        .await
        .unwrap();
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        FIXTURE_CONTROLLER_GENERATION
    );
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn finalizer_enqueued_shutdown_fences_released_successor_before_consumption() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install_released(&fixture).await;
    let controller = fixture.controller();
    let permit = fixture
        .state
        .admit_controller_mutation(controller)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&permit, ExitReason::UserTerminate);
    drop(permit);

    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .arm();
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

    let shutdown = fixture.state.shutdown.notified();
    tokio::pin!(shutdown);
    let state = Arc::clone(&fixture.state);
    tokio::task::spawn_blocking(move || state.notify_termination_shutdown_blocking())
        .await
        .unwrap();
    assert!(fixture.state.response_shutdown_is_committed());

    let mut peer = wire.take_peer();
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();
    tokio::time::timeout(
        ADOPTION_TEST_TIMEOUT,
        acknowledge_exact_worker_release(&mut peer, &offer),
    )
    .await
    .unwrap();
    await_adoption_failure(adoption).await;
    tokio::time::timeout(ADOPTION_TEST_TIMEOUT, &mut shutdown)
        .await
        .unwrap();
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        FIXTURE_CONTROLLER_GENERATION
    );
}
