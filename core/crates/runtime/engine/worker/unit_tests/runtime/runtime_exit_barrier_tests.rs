#[cfg(any(not(test), feature = "yolo-integration-tests"))]
include!("../../integration_tests/runtime/runtime_exit_process_support.rs");

const EXIT_BARRIER_TEST_TIMEOUT: Duration = Duration::from_millis(1);

fn barrier_test_exit(state: &WorkerState) -> ExitRecord {
    ExitRecord {
        runtime_epoch: state.runtime_epoch(),
        code: None,
        signal: Some(libc::SIGHUP),
        reason: ExitReason::Lost,
        observed_at_ms: now_ms(),
        last_output_sequence: lock_recover(&state.journal).last_sequence(),
    }
}

#[tokio::test]
async fn orphan_exit_barrier_observes_durability_after_missing_an_event() {
    let fixture = Fixture::new();
    let exit = barrier_test_exit(&fixture.state);
    fixture
        .state
        .persist_json(
            &fixture.state.paths.exit_record(fixture.terminal_id()),
            &exit,
        )
        .unwrap();
    *lock_recover(&fixture.state.exit) = Some(exit.clone());
    let missed = fixture.state.events.subscribe();
    fixture
        .state
        .events
        .send(WorkerEvent::Exited(exit))
        .unwrap();
    drop(missed);
    let mut events = fixture.state.events.subscribe();

    wait_for_orphan_exit_durability(&fixture.state, &mut events, EXIT_BARRIER_TEST_TIMEOUT)
        .await
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn removed_session_errors_without_durable_exit_publication() {
    let fixture = Fixture::new();
    let manifest = fixture.state.paths.manifest(fixture.terminal_id());
    std::fs::remove_file(&manifest).unwrap();
    let removed = wait_for_session_removal(Arc::clone(&fixture.state)).await;
    assert_eq!(removed.path(), manifest);
    let mut events = fixture.state.events.subscribe();

    let error =
        wait_for_orphan_exit_durability(&fixture.state, &mut events, EXIT_BARRIER_TEST_TIMEOUT)
            .await
            .unwrap_err();

    assert!(error.to_string().contains(ORPHAN_EXIT_DURABILITY_TIMEOUT));
}
