use super::{submit_support::*, *};

#[tokio::test]
async fn submit_rejects_stale_runtime_worker_writer_and_request_before_reservation() {
    let source = prepared();
    enable_paste(&source);
    let request = submit(&source).await;
    let writes = record_writes(&source, WriteBehavior::Complete);
    let mut wrong_writer = request.clone();
    wrong_writer.writer_capability = WriterCapability::generate();
    let mut wrong_generation = request.clone();
    wrong_generation.generation += STEP;
    let mut old_runtime = request.clone();
    old_runtime.runtime_epoch = Uuid::new_v4();
    let mut nil = request.clone();
    nil.request_id = Uuid::nil();
    let mut foreign = request.clone();
    let mut fields = request.witness.fields().clone();
    fields.worker_id = Uuid::new_v4();
    foreign.witness = NativeControlWitness::new(fields).unwrap();
    for invalid in [wrong_writer, wrong_generation, old_runtime, nil, foreign] {
        assert!(
            source
                .fixture
                .state
                .submit_native_agent(source.fixture.controller(), &invalid)
                .await
                .is_err()
        );
    }
    assert_submit_unreserved(&source, &request).await;
    lock_recover(&source.fixture.state.lease).expire_for_test();
    assert!(
        source
            .fixture
            .state
            .submit_native_agent(source.fixture.controller(), &request)
            .await
            .is_err()
    );
    assert_submit_unreserved(&source, &request).await;
    assert!(lock_recover(&writes).is_empty());
}

#[tokio::test]
async fn submit_needs_current_hook_waiting_state_and_bracketed_paste_mode() {
    let mut source = prepared();
    let request = submit(&source).await;
    let writes = record_writes(&source, WriteBehavior::Complete);
    assert!(
        source
            .fixture
            .state
            .submit_native_agent(source.fixture.controller(), &request)
            .await
            .is_err()
    );
    assert_submit_unreserved(&source, &request).await;
    enable_paste(&source);
    source.change_activity();
    assert!(
        source
            .fixture
            .state
            .submit_native_agent(source.fixture.controller(), &request)
            .await
            .is_err()
    );
    assert_submit_unreserved(&source, &request).await;
    source.revoke();
    assert!(
        source
            .fixture
            .state
            .submit_native_agent(source.fixture.controller(), &request)
            .await
            .is_err()
    );
    assert!(lock_recover(&writes).is_empty());
}

#[tokio::test]
async fn native_observation_readiness_comes_from_the_binding_not_the_pty() {
    let source = prepared();
    let state = &source.fixture.state;
    let request = observation(&source);
    let controller = source.fixture.controller();
    let expected =
        source.record.recovery_binding(true).state == crate::agent::AgentState::WaitingForInput;
    let first = state
        .observe_native_agent(controller, &request)
        .await
        .unwrap()
        .unwrap()
        .prompt_input_ready();
    assert_eq!(first, expected);
    enable_paste(&source);
    let second = state
        .observe_native_agent(controller, &request)
        .await
        .unwrap()
        .unwrap()
        .prompt_input_ready();
    assert_eq!(second, expected, "paste mode is a transport precondition");
    lock_recover(&state.terminal).process(PASTE_DISABLED);
    let third = state
        .observe_native_agent(controller, &request)
        .await
        .unwrap()
        .unwrap()
        .prompt_input_ready();
    assert_eq!(third, expected, "paste mode is a transport precondition");
}
