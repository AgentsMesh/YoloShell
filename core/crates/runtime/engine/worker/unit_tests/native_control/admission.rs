use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn source_changed_while_queued_is_rechecked_before_native_admission() {
    let mut source = prepared();
    let request = interrupt(&source).await;
    let state = Arc::clone(&source.fixture.state);
    state.control_test_hooks.controller_before_gate.arm();
    let queued_state = Arc::clone(&state);
    let controller = source.fixture.controller();
    let queued_request = request.clone();
    let queued = tokio::spawn(async move {
        queued_state
            .interrupt_native_agent(controller, &queued_request)
            .await
    });
    tokio::time::timeout(
        ADMISSION_TIMEOUT,
        state
            .control_test_hooks
            .controller_before_gate
            .wait_until_reached(),
    )
    .await
    .unwrap();
    source.change_activity();
    state.control_test_hooks.controller_before_gate.resume();
    assert!(
        tokio::time::timeout(ADMISSION_TIMEOUT, queued)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_unreserved(&source, &request).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writer_taken_over_while_queued_is_rechecked_before_native_admission() {
    let source = prepared();
    let request = interrupt(&source).await;
    let state = Arc::clone(&source.fixture.state);
    state.control_test_hooks.controller_before_gate.arm();
    let queued_state = Arc::clone(&state);
    let controller = source.fixture.controller();
    let queued_request = request.clone();
    let queued = tokio::spawn(async move {
        queued_state
            .interrupt_native_agent(controller, &queued_request)
            .await
    });
    tokio::time::timeout(
        ADMISSION_TIMEOUT,
        state
            .control_test_hooks
            .controller_before_gate
            .wait_until_reached(),
    )
    .await
    .unwrap();
    let mut replacement = source.fixture.lease_request(REPLACEMENT_WRITER);
    replacement.generation = Some(request.generation);
    state
        .acquire_lease(controller, &replacement, true)
        .await
        .unwrap();
    state.control_test_hooks.controller_before_gate.resume();
    assert!(
        tokio::time::timeout(ADMISSION_TIMEOUT, queued)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert_unreserved(&source, &request).await;
}
