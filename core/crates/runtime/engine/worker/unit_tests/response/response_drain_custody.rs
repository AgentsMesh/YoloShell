use crate::worker::tests::*;
#[cfg(target_os = "macos")]
const CUSTODY_DEADLINE_RESET_DETAIL: &str =
    "successor adoption reset its deadline after gate admission";
#[cfg(target_os = "macos")]
const POST_READY_ROLLBACK_TIMEOUT_DETAIL: &str =
    "post-ready custody rollback exceeded its virtual-time budget";

#[cfg(target_os = "macos")]
async fn drive_paused_adoption_and_rollback<T, U>(
    adoption: &tokio::task::JoinHandle<T>,
    rollback: &tokio::task::JoinHandle<U>,
) {
    let deadline = tokio::time::Instant::now()
        + ADOPTION_TEST_TIMEOUT.saturating_add(ADOPTION_TEST_SHORT_TIMEOUT);
    loop {
        tokio::task::yield_now().await;
        if adoption.is_finished() && rollback.is_finished() {
            return;
        }
        let now = tokio::time::Instant::now();
        assert!(now < deadline, "{POST_READY_ROLLBACK_TIMEOUT_DETAIL}");
        tokio::time::advance(ADOPTION_TEST_SHORT_TIMEOUT.min(deadline - now)).await;
    }
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn response_drain_covers_only_the_custody_opening_response() {
    let fixture = Fixture::new();
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let worker = wire.worker.take().unwrap();
    let state = Arc::clone(&fixture.state);
    let mut adoption = tokio::spawn(async move { handle_connection(worker, state).await });
    fixture.state.control_test_hooks.response_before_write.arm();
    wire.peer
        .as_mut()
        .unwrap()
        .write_opening(&wire.request)
        .await
        .unwrap();
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached()
        .await;

    commit_worker_shutdown_for_response_drain(&fixture.state).await;
    assert!(!fixture.state.wait_for_response_drain(Duration::ZERO).await);
    fixture
        .state
        .control_test_hooks
        .response_before_write
        .resume();
    let _offer = wire.receive_offer().await;
    assert!(
        fixture
            .state
            .wait_for_response_drain(RESPONSE_DRAIN_TEST_TIMEOUT)
            .await
    );
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut adoption)
            .await
            .is_err()
    );

    drop(wire.take_peer());
    assert!(adoption.await.unwrap().is_err());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn response_drain_custody_gate_and_opening_share_one_deadline() {
    let fixture = Fixture::new();
    let gate = fixture.state.secondary_custody_adoption_gate.lock().await;
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let mut adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    tokio::task::yield_now().await;
    tokio::time::advance(ADOPTION_TEST_TIMEOUT.saturating_sub(ADOPTION_TEST_SHORT_TIMEOUT)).await;
    drop(gate);
    wire.receive_offer().await;

    tokio::time::advance(ADOPTION_TEST_SHORT_TIMEOUT).await;
    tokio::task::yield_now().await;
    let result = tokio::time::timeout(Duration::ZERO, &mut adoption)
        .await
        .expect(CUSTODY_DEADLINE_RESET_DETAIL)
        .unwrap();
    let error = result.unwrap_err();
    assert!(error.chain().any(|cause| {
        cause.to_string() == secondary_custody_adoption::HANDSHAKE_TIMEOUT_CONTEXT
    }));
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn response_drain_custody_mutation_gate_shares_the_opening_deadline() {
    let fixture = Fixture::new();
    let mutation = Arc::clone(&fixture.state.mutation_gate).lock_owned().await;
    let mut wire = SuccessorAdoptionWire::install(&fixture);
    let adoption = wire.spawn(Arc::clone(&fixture.state), ADOPTION_TEST_TIMEOUT);
    tokio::task::yield_now().await;

    tokio::time::advance(ADOPTION_TEST_TIMEOUT).await;
    tokio::task::yield_now().await;

    let error = adoption.await.unwrap().unwrap_err();
    assert!(error.chain().any(|cause| {
        cause.to_string() == secondary_custody_adoption_deadline::MUTATION_GATE_TIMEOUT_CONTEXT
    }));
    assert!(fixture.state.reserve_response_publication().is_ok());
    drop(mutation);
    drop(wire.take_peer());
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread")]
async fn response_drain_post_ready_gate_timeout_rolls_back_exact_custody() {
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
    let mutation = Arc::clone(&fixture.state.mutation_gate).lock_owned().await;

    tokio::time::pause();
    fixture
        .state
        .control_test_hooks
        .custody_before_promotion
        .resume();
    let mut peer = wire.take_peer();
    let release = tokio::spawn(async move {
        acknowledge_exact_worker_release(&mut peer, &offer).await;
    });
    drive_paused_adoption_and_rollback(&adoption, &release).await;
    release.await.unwrap();

    let error = adoption.await.unwrap().unwrap_err();
    assert!(error.chain().any(|cause| {
        cause.to_string() == secondary_custody_adoption_deadline::MUTATION_GATE_TIMEOUT_CONTEXT
    }));
    drop(mutation);
}
