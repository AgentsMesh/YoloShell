use crate::worker::tests::*;
const DISPLACED_DISPATCH_HOME: &str = "displaced-dispatch-home";
const NAMESPACE_LOSS_REQUEST_ID: &str = "namespace-loss-status";
const CLEANUP_DEFERRED_REQUEST_ID: &str = "cleanup-deferred-termination";
const TERMINATION_INVARIANT_REQUEST_ID: &str = "termination-invariant";
const TEST_EXIT_CODE: i32 = 0;
const TEST_OUTPUT_SEQUENCE: u64 = 0;

#[tokio::test]
async fn worker_error_helpers_and_invalid_payload_reply_are_observable() {
    let fixture = Fixture::new();
    let invalid = RequestEnvelope::new(
        "invalid",
        method::GET_SNAPSHOT,
        yolo_capnp_value::Value::Null,
    );
    let response = dispatch_request(&invalid, &fixture.state, fixture.controller()).await;
    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );

    let (mut client, server) =
        authenticated_worker_connection(&fixture, FIXTURE_CONTROLLER_GENERATION).await;
    let request = RequestEnvelope::new(
        "decode",
        method::GET_SNAPSHOT,
        yolo_capnp_value::Value::Null,
    );
    let response = client.exchange(&request).await.unwrap();
    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INVALID_REQUEST
    );
    drop(client);
    server.await.unwrap().unwrap();

    assert_eq!(protocol_error(ProtocolError::EndOfStream).code, "internal");
    assert_eq!(internal_error("failure").message, "failure");
    assert_eq!(signal_number("TERM"), Some(libc::SIGTERM));
    assert_eq!(signal_number("unknown"), None);
    assert!(validate_socket_path(Path::new("/tmp/worker.sock")).is_ok());
    assert!(validate_socket_path(Path::new(&format!("/{}", "x".repeat(110)))).is_err());

    let exit = ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(TEST_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: TEST_OUTPUT_SEQUENCE,
    };
    atomic_write_json(
        &fixture.state.paths.exit_record(fixture.terminal_id()),
        &exit,
    )
    .unwrap();
    assert!(load_exit(&fixture.state.paths, fixture.terminal_id()).is_some());
    assert!(load_snapshot(&fixture.state.paths, Uuid::new_v4()).is_none());
}

#[tokio::test]
async fn dispatch_rejects_a_lost_worker_namespace_before_execution() {
    let fixture = Fixture::new();
    let displaced = fixture._filesystem.scratch_path(DISPLACED_DISPATCH_HOME);
    std::fs::rename(fixture.state.paths.home(), displaced).unwrap();
    let request = RequestEnvelope::new(
        NAMESPACE_LOSS_REQUEST_ID,
        WORKER_STATUS_METHOD,
        yolo_capnp_value::Value::Null,
    );

    let response = dispatch_request(&request, &fixture.state, fixture.controller()).await;

    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INTERNAL
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn dispatch_rejects_namespace_loss_during_execution() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .snapshot_before_publication
        .arm();
    let state = Arc::clone(&fixture.state);
    let controller = fixture.controller();
    let request = RequestEnvelope::new(
        NAMESPACE_LOSS_REQUEST_ID,
        method::GET_SNAPSHOT,
        to_value(&SnapshotRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
        })
        .unwrap(),
    );
    let dispatch =
        tokio::spawn(async move { dispatch_request(&request, &state, controller).await });
    fixture
        .state
        .control_test_hooks
        .snapshot_before_publication
        .wait_until_reached();
    let displaced = fixture._filesystem.scratch_path(DISPLACED_DISPATCH_HOME);
    std::fs::rename(fixture.state.paths.home(), displaced).unwrap();
    fixture
        .state
        .control_test_hooks
        .snapshot_before_publication
        .resume();

    let response = dispatch.await.unwrap();
    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INTERNAL
    );
}

#[tokio::test]
async fn dispatch_fails_closed_if_an_exited_runtime_loses_its_exit_record() {
    let fixture = Fixture::new();
    *lock_recover(&fixture.state.exit) = Some(ExitRecord {
        runtime_epoch: fixture.state.runtime_epoch(),
        code: Some(TEST_EXIT_CODE),
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: TEST_OUTPUT_SEQUENCE,
    });
    fixture
        .state
        .control_test_hooks
        .termination_after_tombstone
        .arm();
    let state = Arc::clone(&fixture.state);
    let controller = fixture.controller();
    let request = RequestEnvelope::new(
        TERMINATION_INVARIANT_REQUEST_ID,
        method::TERMINATE_RUNTIME_EXACT,
        to_value(&FencedTerminateRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
            expected_status: None,
            force: false,
        })
        .unwrap(),
    );
    let dispatch =
        tokio::spawn(async move { dispatch_request(&request, &state, controller).await });
    fixture
        .state
        .control_test_hooks
        .termination_after_tombstone
        .wait_until_reached()
        .await;
    *lock_recover(&fixture.state.exit) = None;
    fixture
        .state
        .control_test_hooks
        .termination_after_tombstone
        .resume();

    let response = dispatch.await.unwrap();
    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::INTERNAL
    );
}

include!("cleanup_errors.rs");
