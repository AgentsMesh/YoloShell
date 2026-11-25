const RUNTIME_STATUS_TIMEOUT_DETAIL: &str = "runtime status did not converge before its deadline";

async fn dispatch_test<T: serde::Serialize>(
    state: &Arc<WorkerState>,
    method_name: &str,
    body: &T,
) -> std::result::Result<ResponseEnvelope, ErrorBody> {
    let request = RequestEnvelope::new(
        Uuid::new_v4().to_string(),
        method_name,
        to_value(body).unwrap(),
    );
    dispatch_request_inner(&request, state, state.current_authenticated_controller()).await
}

async fn wait_for_runtime_status(state: &WorkerState, expected: TerminalRuntimeStatus, timeout: Duration) {
    let result = tokio::time::timeout(timeout, async {
        while state.status().status != expected {
            tokio::task::yield_now().await;
        }
    })
    .await;
    assert!(
        result.is_ok(),
        "{RUNTIME_STATUS_TIMEOUT_DETAIL}: expected {expected:?}, actual {:?}, child waited {}, output pending {}, reader {:?}",
        state.status().status,
        state.child_waited.load(Ordering::Acquire),
        state.reader_lifecycle.has_pending(),
        *state.reader_lifecycle.lock_progress(),
    );
}

async fn assert_dispatch_termination(fixture: &Fixture, state: &Arc<WorkerState>) {
    let release = fixture.start_termination_runtime();
    let terminate = FencedTerminateRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        expected_status: None,
        force: false,
    };
    dispatch_test(state, method::TERMINATE_RUNTIME_EXACT, &terminate)
        .await
        .unwrap();
    assert_eq!(state.termination_reason(), Some(ExitReason::UserTerminate));
    let tombstone: TerminationTombstone =
        read_json(&state.paths.terminate_tombstone(fixture.terminal_id())).unwrap();
    assert_eq!(
        tombstone.validate_for(fixture.terminal_id(), state.runtime_epoch()),
        Ok(())
    );
    assert!(!tombstone.force);
    wait_for_runtime_status(state, TerminalRuntimeStatus::Exited, WORKER_TERMINATION_TIMEOUT).await;
    if let Some(release) = release {
        release.finish().await;
    }
    let original_exit = state.exit.lock().unwrap().clone().unwrap();
    let exit_path = state.paths.exit_record(fixture.terminal_id());
    assert_eq!(original_exit.runtime_epoch, state.runtime_epoch());
    assert_eq!(read_json::<ExitRecord>(&exit_path).unwrap(), original_exit);
    dispatch_test(
        state,
        method::TERMINATE_RUNTIME_EXACT,
        &FencedTerminateRequest {
            force: true,
            ..terminate
        },
    )
    .await
    .unwrap();
    let forced_tombstone: TerminationTombstone =
        read_json(&state.paths.terminate_tombstone(fixture.terminal_id())).unwrap();
    assert!(forced_tombstone.force);
    assert_eq!(forced_tombstone.runtime_epoch, state.runtime_epoch());
    assert_eq!(
        forced_tombstone.validate_for(fixture.terminal_id(), state.runtime_epoch()),
        Ok(())
    );
    assert_eq!(*state.exit.lock().unwrap(), Some(original_exit.clone()));
    assert_eq!(read_json::<ExitRecord>(&exit_path).unwrap(), original_exit);
}
