use crate::worker::tests::*;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn resize_side_effect_finishes_before_controller_adoption_ack() {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(old_controller, &fixture.lease_request("resize"), false)
        .await
        .unwrap();
    let (reached, release, sizes) = install_blocking_master(&fixture.state);
    let request = ResizeRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        generation: lease.generation,
        writer_capability: lease.writer_capability.clone(),
        cols: BARRIER_TEST_COLS,
        rows: BARRIER_TEST_ROWS,
    };
    let resize_state = Arc::clone(&fixture.state);
    let resize_request = request.clone();
    let resize = tokio::spawn(async move {
        resize_state
            .resize_terminal(old_controller, &resize_request)
            .await
    });
    wait_for_sync_signal(reached).await;

    let adoption_state = Arc::clone(&fixture.state);
    let adoption = tokio::spawn(async move { adoption_state.adopt_controller(2).await });
    assert_task_pending(&adoption).await;
    release.send(()).unwrap();
    resize.await.unwrap().unwrap();
    adoption.await.unwrap().unwrap();

    assert_eq!(
        *lock_recover(&fixture.state.dimensions),
        (BARRIER_TEST_COLS, BARRIER_TEST_ROWS)
    );
    assert!(
        fixture
            .state
            .resize_terminal(old_controller, &request)
            .await
            .is_err()
    );
    assert_eq!(lock_recover(&sizes).len(), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn signal_delivery_finishes_before_controller_adoption_ack() {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(old_controller, &fixture.lease_request("signal"), false)
        .await
        .unwrap();
    fixture
        .state
        .control_test_hooks
        .signal_before_delivery
        .arm();
    let signal_state = Arc::clone(&fixture.state);
    let lease_generation = lease.generation;
    let signal_capability = lease.writer_capability.clone();
    let stale_capability = lease.writer_capability.clone();
    let signal = tokio::spawn(async move {
        let permit = signal_state
            .admit_writer_mutation(old_controller, lease_generation, &signal_capability)
            .await?;
        let signal = PtyForegroundSignal::parse(BARRIER_TEST_SIGNAL).map_err(internal_error)?;
        signal_pty_foreground(&signal_state, &permit, signal)
            .await
            .map_err(internal_error)
    });
    fixture
        .state
        .control_test_hooks
        .signal_before_delivery
        .wait_until_reached()
        .await;

    let adoption_state = Arc::clone(&fixture.state);
    let adoption = tokio::spawn(async move { adoption_state.adopt_controller(2).await });
    assert_task_pending(&adoption).await;
    fixture
        .state
        .control_test_hooks
        .signal_before_delivery
        .resume();
    signal.await.unwrap().unwrap();
    adoption.await.unwrap().unwrap();
    assert!(
        fixture
            .state
            .admit_writer_mutation(old_controller, lease_generation, &stale_capability)
            .await
            .is_err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn termination_owner_wait_releases_the_mutation_gate() {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    fixture
        .state
        .control_test_hooks
        .terminate_before_delivery
        .arm();
    let terminate_state = Arc::clone(&fixture.state);
    let terminate = tokio::spawn(async move {
        let permit = terminate_state
            .admit_controller_mutation(old_controller)
            .await?;
        terminate_pty_processes(&terminate_state, permit, true)
            .await
            .map_err(internal_error)
    });
    fixture
        .state
        .control_test_hooks
        .terminate_before_delivery
        .wait_until_reached()
        .await;

    let permit = tokio::time::timeout(
        CONTROL_BARRIER_TIMEOUT,
        fixture.state.admit_system_mutation(),
    )
    .await
    .expect(TERMINATION_GATE_RELEASE_DETAIL);
    drop(permit);
    assert_task_pending(&terminate).await;
    fixture
        .state
        .control_test_hooks
        .terminate_before_delivery
        .resume();
    terminate.await.unwrap().unwrap();
}

#[tokio::test(start_paused = true)]
async fn delayed_shutdown_revalidates_controller_generation() {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    let notified = fixture.state.shutdown.notified();
    tokio::pin!(notified);
    fixture.state.schedule_controller_shutdown(old_controller);
    tokio::task::yield_now().await;
    fixture.state.adopt_controller(2).await.unwrap();

    tokio::time::advance(EXPLICIT_EXIT_SHUTDOWN_DELAY).await;
    tokio::task::yield_now().await;
    assert!(
        tokio::time::timeout(Duration::ZERO, &mut notified)
            .await
            .is_err()
    );
}
