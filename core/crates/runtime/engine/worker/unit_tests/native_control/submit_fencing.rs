use super::{submit_support::*, *};

#[tokio::test]
async fn stale_source_writer_or_paste_mode_after_reservation_never_delivers() {
    for fault in [
        Fault::WriterExpiry,
        Fault::PasteDisabled,
        Fault::HookChanged,
    ] {
        let source = prepared();
        enable_paste(&source);
        let request = submit(&source).await;
        let writes = record_writes(&source, WriteBehavior::Complete);
        let state = Arc::clone(&source.fixture.state);
        source
            .fixture
            .state
            .native_control_test_hooks
            .after_next_reserve(move || match fault {
                Fault::WriterExpiry => lock_recover(&state.lease).expire_for_test(),
                Fault::PasteDisabled => lock_recover(&state.terminal).process(PASTE_DISABLED),
                Fault::HookChanged => {
                    let path = state.paths.agent_binding(state.terminal_id());
                    let mut binding: crate::agent::AgentBinding =
                        state.namespace.read_json(&path).unwrap();
                    binding.state = crate::agent::AgentState::Running;
                    std::fs::write(path, serde_json::to_vec(&binding).unwrap()).unwrap();
                }
            });
        let result = source
            .fixture
            .state
            .submit_native_agent(source.fixture.controller(), &request)
            .await
            .unwrap();
        assert_eq!(result.outcome, NativeControlOutcome::Unknown);
        assert!(lock_recover(&writes).is_empty());
        assert_eq!(
            source
                .fixture
                .state
                .native_control_receipt(source.fixture.controller(), &submit_lookup(&request),)
                .await
                .unwrap(),
            Some(result)
        );
    }
}

enum Fault {
    WriterExpiry,
    PasteDisabled,
    HookChanged,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn writer_takeover_while_prompt_is_queued_prevents_delivery() {
    let source = prepared();
    enable_paste(&source);
    let request = submit(&source).await;
    let writes = record_writes(&source, WriteBehavior::Complete);
    let state = Arc::clone(&source.fixture.state);
    state.control_test_hooks.controller_before_gate.arm();
    let queued_state = Arc::clone(&state);
    let controller = source.fixture.controller();
    let queued_request = request.clone();
    let queued = tokio::spawn(async move {
        queued_state
            .submit_native_agent(controller, &queued_request)
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
    assert_submit_unreserved(&source, &request).await;
    assert!(lock_recover(&writes).is_empty());
}
