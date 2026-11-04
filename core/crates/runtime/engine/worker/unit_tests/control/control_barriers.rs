use crate::worker::tests::*;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn controller_adoption_ack_waits_for_admitted_input_side_effect() {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(old_controller, &fixture.lease_request("old"), false)
        .await
        .unwrap();
    let (reached, release, writes) = install_blocking_writer(&fixture.state);
    let input_state = Arc::clone(&fixture.state);
    let request = barrier_input(&fixture, &lease);
    let input =
        tokio::spawn(async move { input_state.write_input(old_controller, &request).await });
    wait_for_sync_signal(reached).await;

    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let adoption_state = Arc::clone(&fixture.state);
    let adoption = tokio::spawn(async move {
        let _ = started_tx.send(());
        adoption_state.adopt_controller(2).await
    });
    started_rx.await.unwrap();
    assert_task_pending(&adoption).await;

    release.send(()).unwrap();
    input.await.unwrap().unwrap();
    adoption.await.unwrap().unwrap();
    let stale = barrier_input(&fixture, &lease);
    assert!(
        fixture
            .state
            .write_input(old_controller, &stale)
            .await
            .is_err()
    );
    assert_eq!(writes.load(Ordering::Acquire), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn queued_old_request_revalidates_after_controller_adoption() {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(old_controller, &fixture.lease_request("queued"), false)
        .await
        .unwrap();
    let writes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    *lock_recover(&fixture.state.writer) = Some(PtyInputWriter::test(SharedInputWriter {
        bytes: Arc::new(Mutex::new(Vec::new())),
        writes: Arc::clone(&writes),
        fail: false,
    }));
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .arm();
    let state = Arc::clone(&fixture.state);
    let request = barrier_input(&fixture, &lease);
    let old_request =
        tokio::spawn(async move { state.write_input(old_controller, &request).await });
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .wait_until_reached()
        .await;

    fixture.state.adopt_controller(2).await.unwrap();
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .resume();
    assert!(old_request.await.unwrap().is_err());
    assert_eq!(writes.load(Ordering::Acquire), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lease_takeover_ack_waits_for_old_holder_input() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request("first"), false)
        .await
        .unwrap();
    let mut events = fixture.state.events.subscribe();
    let (reached, release, writes) = install_blocking_writer(&fixture.state);
    let input_state = Arc::clone(&fixture.state);
    let request = barrier_input(&fixture, &lease);
    let input = tokio::spawn(async move { input_state.write_input(controller, &request).await });
    wait_for_sync_signal(reached).await;

    let mut takeover_request = fixture.lease_request("second");
    takeover_request.generation = Some(lease.generation);
    let takeover_state = Arc::clone(&fixture.state);
    let takeover = tokio::spawn(async move {
        takeover_state
            .acquire_lease(controller, &takeover_request, true)
            .await
    });
    assert_task_pending(&takeover).await;
    assert!(events.try_recv().is_err());

    release.send(()).unwrap();
    input.await.unwrap().unwrap();
    let replacement = takeover.await.unwrap().unwrap();
    assert!(replacement.generation > lease.generation);
    assert!(matches!(
        events.recv().await,
        Ok(WorkerEvent::WriterLease {
            event_type: event::WRITER_LEASE_REVOKED,
            ..
        })
    ));
    assert!(matches!(
        events.recv().await,
        Ok(WorkerEvent::WriterLease {
            event_type: event::WRITER_LEASE_ACQUIRED,
            ..
        })
    ));
    let stale = barrier_input(&fixture, &lease);
    assert!(fixture.state.write_input(controller, &stale).await.is_err());
    assert_eq!(writes.load(Ordering::Acquire), 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn lease_expiry_event_waits_for_admitted_input() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(controller, &fixture.lease_request("expiring"), false)
        .await
        .unwrap();
    let mut events = fixture.state.events.subscribe();
    let (reached, release, writes) = install_blocking_writer(&fixture.state);
    let input_state = Arc::clone(&fixture.state);
    let request = barrier_input(&fixture, &lease);
    let input = tokio::spawn(async move { input_state.write_input(controller, &request).await });
    wait_for_sync_signal(reached).await;
    lock_recover(&fixture.state.lease).expire_for_test();

    let lease_state = Arc::clone(&fixture.state);
    let expiry = tokio::spawn(async move { lease_state.get_writer_lease(controller).await });
    assert_task_pending(&expiry).await;
    assert!(events.try_recv().is_err());

    release.send(()).unwrap();
    input.await.unwrap().unwrap();
    assert!(expiry.await.unwrap().unwrap().is_none());
    assert!(matches!(
        events.recv().await,
        Ok(WorkerEvent::WriterLease {
            event_type: event::WRITER_LEASE_EXPIRED,
            ..
        })
    ));
    let stale = barrier_input(&fixture, &lease);
    assert!(fixture.state.write_input(controller, &stale).await.is_err());
    assert_eq!(writes.load(Ordering::Acquire), 1);
}
