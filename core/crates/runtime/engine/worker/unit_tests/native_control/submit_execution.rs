use super::{submit_support::*, *};

#[tokio::test]
async fn prompt_delivers_one_bounded_paste_and_duplicate_does_not_write_again() {
    let source = prepared();
    enable_paste(&source);
    let request = submit(&source).await;
    let writes = record_writes(&source, WriteBehavior::Complete);
    let state = &source.fixture.state;
    let controller = source.fixture.controller();
    let first = state
        .submit_native_agent(controller, &request)
        .await
        .unwrap();
    assert_eq!(first.outcome, NativeControlOutcome::Delivered);
    assert_eq!(*lock_recover(&writes), [PAYLOAD.to_vec()]);
    source.revoke();
    assert_eq!(
        state
            .submit_native_agent(controller, &request)
            .await
            .unwrap(),
        first
    );
    assert_eq!(lock_recover(&writes).len(), ONE_WRITE);
    let mut collision = request.clone();
    collision.command_digest = yolo_crypto::Sha256Digest::capture(&OTHER_COMMAND).unwrap();
    assert!(
        state
            .submit_native_agent(controller, &collision)
            .await
            .is_err()
    );
    assert_eq!(lock_recover(&writes).len(), ONE_WRITE);
    lock_recover(&state.lease).expire_for_test();
    assert_eq!(
        state
            .native_control_receipt(controller, &submit_lookup(&request))
            .await
            .unwrap(),
        Some(first)
    );
}

#[tokio::test]
async fn partial_blocked_and_interrupted_input_leave_durable_unknown_without_replay() {
    for behavior in [
        WriteBehavior::Partial,
        WriteBehavior::Blocked,
        WriteBehavior::Interrupted,
    ] {
        let source = prepared();
        enable_paste(&source);
        let request = submit(&source).await;
        let writes = record_writes(&source, behavior);
        let state = &source.fixture.state;
        let controller = source.fixture.controller();
        let first = state
            .submit_native_agent(controller, &request)
            .await
            .unwrap();
        assert_eq!(first.outcome, NativeControlOutcome::Unknown);
        assert_eq!(lock_recover(&writes).len(), ONE_WRITE);
        // Reopening the actual durable journal must not turn uncertainty into a new intent.
        let runtime = lock_recover(&state.record).clone();
        drop(lock_recover(&state.native_control).take());
        let directory = state
            .namespace
            .lease()
            .directory(&state.paths.session_dir(runtime.terminal_id))
            .unwrap();
        *lock_recover(&state.native_control) = Some(
            yolo_agent_control_runtime::ControlRuntime::open_existing(
                directory,
                &state.paths,
                &runtime,
            )
            .unwrap(),
        );
        assert_eq!(
            state
                .native_control_receipt(controller, &submit_lookup(&request))
                .await
                .unwrap(),
            Some(first.clone())
        );
        assert_eq!(
            state
                .submit_native_agent(controller, &request)
                .await
                .unwrap(),
            first
        );
        assert_eq!(lock_recover(&writes).len(), ONE_WRITE);
    }
}

#[tokio::test]
async fn closed_writer_after_reservation_is_unknown_and_is_never_retried() {
    let source = prepared();
    enable_paste(&source);
    let request = submit(&source).await;
    *lock_recover(&source.fixture.state.writer) = None;
    let first = source
        .fixture
        .state
        .submit_native_agent(source.fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(first.outcome, NativeControlOutcome::Unknown);
    let writes = record_writes(&source, WriteBehavior::Complete);
    assert_eq!(
        source
            .fixture
            .state
            .submit_native_agent(source.fixture.controller(), &request)
            .await
            .unwrap(),
        first
    );
    assert!(lock_recover(&writes).is_empty());
}

fn deferred_registry(
    gap: std::time::Duration,
) -> yolo_agent_providers::ProviderRuntimeRegistry {
    let builtin = yolo_agent_providers::builtin_provider_runtime();
    let runtimes = builtin
        .runtimes()
        .map(|runtime| {
            if *runtime.descriptor().kind() == crate::agent::AgentKind::ClaudeCode {
                runtime
                    .clone()
                    .with_prompt_submit(yolo_agent_providers::PromptSubmit::DeferredEnter { gap })
            } else {
                runtime.clone()
            }
        });
    yolo_agent_providers::ProviderRuntimeRegistry::from_runtimes(runtimes).unwrap()
}

#[tokio::test]
async fn deferred_provider_settles_the_paste_before_the_submit_key() {
    const GAP: std::time::Duration = std::time::Duration::from_millis(20);
    let source = prepared_with_provider_runtime(deferred_registry(GAP));
    enable_paste(&source);
    let request = submit(&source).await;
    let writes = record_writes(&source, WriteBehavior::Complete);
    let started = std::time::Instant::now();
    let result = source
        .fixture
        .state
        .submit_native_agent(source.fixture.controller(), &request)
        .await
        .unwrap();
    assert_eq!(result.outcome, NativeControlOutcome::Delivered);
    assert!(started.elapsed() >= GAP);
    assert_eq!(
        *lock_recover(&writes),
        [PASTE_ONLY.to_vec(), SUBMIT_KEY.to_vec()]
    );
}
