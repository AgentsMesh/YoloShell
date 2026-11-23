use crate::worker::tests::*;
#[tokio::test]
async fn exact_termination_revalidates_status_after_mutation_admission_wait() {
    let fixture = Fixture::new();
    let request = exact_termination_request(&fixture, TerminalRuntimeStatus::Running);
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .arm();
    let state = Arc::clone(&fixture.state);
    let dispatch = tokio::spawn(async move {
        dispatch_test(&state, method::TERMINATE_RUNTIME_EXACT, &request).await
    });
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .wait_until_reached()
        .await;

    fixture
        .state
        .mark_output_blocked(OutputBlockCause::ExitPersistence);
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .resume();

    let error = dispatch.await.unwrap().unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_termination_was_not_started(&fixture);
}

#[tokio::test]
async fn exact_termination_revalidates_epoch_after_mutation_admission_wait() {
    let fixture = Fixture::new();
    let request = exact_termination_request(&fixture, TerminalRuntimeStatus::Running);
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .arm();
    let state = Arc::clone(&fixture.state);
    let dispatch = tokio::spawn(async move {
        dispatch_test(&state, method::TERMINATE_RUNTIME_EXACT, &request).await
    });
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .wait_until_reached()
        .await;

    lock_recover(&fixture.state.record).runtime_epoch = Uuid::new_v4();
    fixture
        .state
        .control_test_hooks
        .controller_before_gate
        .resume();

    let error = dispatch.await.unwrap().unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert_termination_was_not_started(&fixture);
}
