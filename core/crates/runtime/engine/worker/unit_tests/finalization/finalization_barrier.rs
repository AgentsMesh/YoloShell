use crate::worker::tests::*;
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn finalizer_resource_mutations_wait_for_admitted_input() {
    let scenario = FinalizerProcessScenario::ResourceMutations;
    if scenario.is_child() {
        finalizer_resource_mutations_inner().await;
    } else {
        FinalizerTestProcess::run(scenario);
    }
}

async fn finalizer_resource_mutations_inner() {
    let fixture = Fixture::new();
    let controller = fixture.controller();
    let lease = fixture
        .state
        .acquire_lease(
            controller,
            &fixture.lease_request(FINALIZER_LEASE_ID),
            false,
        )
        .await
        .unwrap();
    let mut events = fixture.state.events.subscribe();
    let (reached, release, writes) = install_blocking_writer(&fixture.state);
    let input_state = Arc::clone(&fixture.state);
    let request = barrier_input(&fixture, &lease);
    let input = BoundedTestTask::spawn(FINALIZER_INPUT_TASK_LABEL, async move {
        input_state.write_input(controller, &request).await
    });
    wait_for_sync_signal(reached).await;
    fixture.state.reader_lifecycle.mark_drained();

    let mut finalizer = FinalizerThread::spawn(
        Arc::clone(&fixture.state),
        FinalizerRelease::Writer(release),
    );
    finalizer.wait_until_system_admission().await;
    let child_waited_before_release = fixture.state.child_waited.load(Ordering::Acquire);
    let writer_locked_before_release = fixture.state.writer.try_lock().is_err();
    let master_present_before_release = lock_recover(&fixture.state.master).is_some();
    let event_pending_before_release = events.try_recv().is_err();

    finalizer.release_gate();
    input.finish().await.unwrap();
    finalizer.finish();
    assert!(!child_waited_before_release);
    assert!(writer_locked_before_release);
    assert!(master_present_before_release);
    assert!(event_pending_before_release);
    assert_eq!(writes.load(Ordering::Acquire), 1);
    assert!(fixture.state.child_waited.load(Ordering::Acquire));
    assert!(lock_recover(&fixture.state.writer).is_none());
    assert!(lock_recover(&fixture.state.master).is_none());
    let lease_error = fixture
        .state
        .get_writer_lease(controller)
        .await
        .unwrap_err();
    assert_eq!(lease_error.code, lease_error_code::SESSION_NOT_RUNNING);
    let event = tokio::time::timeout(CONTROL_BARRIER_TIMEOUT, events.recv())
        .await
        .expect(FINALIZER_EVENT_TIMEOUT_DETAIL);
    assert!(matches!(
        event,
        Ok(WorkerEvent::WriterLease {
            event_type: event::WRITER_LEASE_REVOKED,
            ..
        })
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pending_controller_finalizer_fences_adoption_and_commits_shutdown() {
    let scenario = FinalizerProcessScenario::ControllerFencing;
    if scenario.is_child() {
        pending_controller_finalizer_inner(Duration::ZERO).await;
    } else {
        FinalizerTestProcess::run(scenario);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn delayed_finalizer_controller_shutdown_uses_process_deadline() {
    let scenario = FinalizerProcessScenario::DelayedControllerFencing;
    if scenario.is_child() {
        let delay = CONTROL_BARRIER_TIMEOUT.saturating_add(CONTROL_BARRIER_TIMEOUT);
        pending_controller_finalizer_inner(delay).await;
    } else {
        FinalizerTestProcess::run(scenario);
    }
}

async fn pending_controller_finalizer_inner(delay: Duration) {
    let fixture = Fixture::new();
    let old_controller = fixture.controller();
    let permit = fixture
        .state
        .admit_controller_mutation(old_controller)
        .await
        .unwrap();
    fixture
        .state
        .begin_controller_termination(&permit, ExitReason::UserTerminate);
    drop(permit);
    fixture.state.reader_lifecycle.mark_drained();
    fixture
        .state
        .control_test_hooks
        .finalizer_before_shutdown
        .arm();
    let notified = fixture.state.shutdown.notified();
    tokio::pin!(notified);

    let mut finalizer = FinalizerThread::spawn_delayed(
        Arc::clone(&fixture.state),
        FinalizerRelease::ShutdownHook,
        delay,
    );
    finalizer.wait_until_shutdown_hook().await;

    let custody = ControllerCustodySnapshot::capture(&fixture);
    let adoption = tokio::time::timeout(
        CONTROL_BARRIER_TIMEOUT,
        fixture.state.adopt_controller(CUSTODY_SUCCESSOR_GENERATION),
    )
    .await
    .expect(FINALIZER_ADOPTION_TIMEOUT_DETAIL)
    .unwrap_err();
    let custody_after_adoption = ControllerCustodySnapshot::capture(&fixture);
    finalizer.finish();
    custody.assert_termination_intent_rejection(custody_after_adoption, adoption);
    tokio::time::timeout(CONTROL_BARRIER_TIMEOUT, &mut notified)
        .await
        .unwrap();
    assert!(fixture.state.response_shutdown_is_committed());
}
