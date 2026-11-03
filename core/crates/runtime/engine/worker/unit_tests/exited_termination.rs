use crate::worker::tests::*;
const EXITED_AT_ENTRY_CODE: i32 = 0;

#[tokio::test(start_paused = true)]
async fn exited_at_entry_termination_persists_intent_before_delayed_shutdown() {
    let fixture = Fixture::new();
    let state = Arc::clone(&fixture.state);
    let exit_path = state.paths.exit_record(fixture.terminal_id());
    let original_exit = ExitRecord {
        runtime_epoch: state.runtime_epoch(),
        code: Some(EXITED_AT_ENTRY_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: lock_recover(&state.journal).last_sequence(),
    };
    state.persist_json(&exit_path, &original_exit).unwrap();
    *lock_recover(&state.exit) = Some(original_exit.clone());
    state.control_test_hooks.termination_after_tombstone.arm();
    let mut shutdown = Box::pin(state.shutdown.notified());
    let request = FencedTerminateRequest {
        terminal_id: fixture.terminal_id(),
        runtime_epoch: fixture.state.runtime_epoch(),
        expected_status: None,
        force: true,
    };
    let termination = ControlledTerminationTask::spawn(Arc::clone(&state), request)
        .wait_until_tombstone()
        .await;
    let tombstone =
        read_json::<TerminationTombstone>(&state.paths.terminate_tombstone(fixture.terminal_id()));
    let exit_before_release = lock_recover(&state.exit).clone();
    let persisted_exit_before_release = read_json::<ExitRecord>(&exit_path);
    let shutdown_committed_before_release = state.response_shutdown_is_committed();
    let shutdown_before_release = tokio::time::timeout(Duration::ZERO, &mut shutdown).await;
    termination.finish().await.unwrap();

    let tombstone = tombstone.unwrap();
    assert!(tombstone.force);
    assert_eq!(tombstone.runtime_epoch, state.runtime_epoch());
    assert_eq!(
        tombstone.validate_for(fixture.terminal_id(), state.runtime_epoch()),
        Ok(())
    );
    assert_eq!(exit_before_release, Some(original_exit.clone()));
    assert_eq!(persisted_exit_before_release.unwrap(), original_exit);
    assert!(!shutdown_committed_before_release);
    assert!(shutdown_before_release.is_err());
    tokio::task::yield_now().await;
    assert!(!state.response_shutdown_is_committed());

    tokio::time::advance(EXPLICIT_EXIT_SHUTDOWN_DELAY).await;
    tokio::task::yield_now().await;
    tokio::time::timeout(Duration::ZERO, &mut shutdown)
        .await
        .unwrap();
    assert!(state.response_shutdown_is_committed());
    assert_eq!(*lock_recover(&state.exit), Some(original_exit.clone()));
    assert_eq!(read_json::<ExitRecord>(&exit_path).unwrap(), original_exit);
}
