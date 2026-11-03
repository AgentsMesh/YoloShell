use crate::worker::tests::*;
#[tokio::test]
async fn dispatch_reads_empty_lease_and_rejects_zero_rows() {
    let fixture = Fixture::new();
    let response = dispatch_test(
        &fixture.state,
        method::GET_WRITER_LEASE,
        &RuntimeRef::new(fixture.terminal_id(), fixture.state.runtime_epoch()),
    )
    .await
    .unwrap();
    assert!(response.ok);

    let lease = fixture
        .state
        .acquire_lease(
            fixture.controller(),
            &fixture.lease_request("resize"),
            false,
        )
        .await
        .unwrap();
    let error = dispatch_test(
        &fixture.state,
        method::RESIZE_TERMINAL,
        &ResizeRequest {
            terminal_id: fixture.terminal_id(),
            runtime_epoch: fixture.state.runtime_epoch(),
            generation: lease.generation,
            writer_capability: lease.writer_capability.clone(),
            cols: 80,
            rows: 0,
        },
    )
    .await
    .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INVALID_REQUEST);
}

#[tokio::test]
async fn authentication_without_error_body_uses_stable_fallback() {
    let directory = tempfile::tempdir().unwrap();
    let socket_path = directory.path().join("worker.sock");
    let listener = UnixListener::bind(&socket_path).unwrap();
    set_current_worker_socket_permissions(&socket_path).unwrap();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let response = ResponseEnvelope {
            version: crate::protocol::PROTOCOL_VERSION,
            kind: "response".into(),
            request_id: request.request_id,
            method: request.method,
            ok: false,
            payload: yolo_capnp_value::Value::Null,
            error: None,
        };
        write_frame(&mut stream, &response).await.unwrap();
    });
    let fixture = Fixture::new();
    let mut record = fixture.state.record.lock().unwrap().clone();
    record.socket_path = socket_path;
    let error = authenticated_stream(&record, 1).await.unwrap_err();
    assert!(matches!(
        error,
        WorkerRpcError::Protocol(detail)
            if detail == ProtocolError::ResponseMismatch.to_string()
    ));
    server.await.unwrap();
}

#[tokio::test]
async fn protocol_helpers_cover_permission_edges() {
    let missing = tempfile::tempdir().unwrap().path().join("missing.sock");
    assert!(set_current_worker_socket_permissions(&missing).is_err());
    assert!(validate_socket_path(std::path::Path::new("short.sock")).is_ok());
    assert_eq!(signal_number("definitely-not-a-signal"), None);

    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let _namespace = paths.retain_socket_namespace().unwrap();
    let live_socket = paths.worker_socket(terminal_id).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&live_socket).unwrap();
    set_current_worker_socket_permissions(&live_socket).unwrap();
    assert!(
        bind_worker_socket(&paths, terminal_id, &live_socket)
            .await
            .is_err()
    );
    assert!(std::os::unix::net::UnixStream::connect(&live_socket).is_ok());
    drop(listener);
}

#[tokio::test]
async fn controller_generation_persistence_failure_is_fenced_and_retryable() {
    let fixture = Fixture::new();
    fixture
        .state
        .control_test_hooks
        .fail_next_controller_persist();
    assert!(fixture.state.adopt_controller(2).await.is_err());
    assert_eq!(
        fixture.state.controller_generation.load(Ordering::Acquire),
        FIXTURE_CONTROLLER_GENERATION
    );
    assert_eq!(
        fixture
            .state
            .controller_persisted_generation
            .load(Ordering::Acquire),
        FIXTURE_CONTROLLER_GENERATION
    );
    assert_eq!(
        lock_recover(&fixture.state.record).controller_generation,
        FIXTURE_CONTROLLER_GENERATION
    );
    let unchanged: RuntimeRecord =
        read_json(&fixture.state.paths.runtime_record(fixture.terminal_id())).unwrap();
    assert_eq!(
        unchanged.controller_generation,
        FIXTURE_CONTROLLER_GENERATION
    );

    let controller = fixture.state.adopt_controller(2).await.unwrap();
    assert!(fixture.state.validate_controller(controller).is_ok());
    let record: RuntimeRecord =
        read_json(&fixture.state.paths.runtime_record(fixture.terminal_id())).unwrap();
    assert_eq!(record.controller_generation, 2);
}

#[tokio::test]
async fn worker_operation_deadline_contains_a_stalled_socket() {
    let error = worker_operation_with_timeout(IMMEDIATE_WORKER_TIMEOUT, async {
        std::future::pending::<Result<()>>().await
    })
    .await
    .unwrap_err();
    assert!(error.to_string().contains(WORKER_OPERATION_TIMEOUT_CONTEXT));
}

#[test]
fn worker_deadlines_exceed_the_complete_termination_drain() {
    let expected_drain = GRACEFUL_TERMINATION_TIMEOUT
        .saturating_add(GRACEFUL_TERMINATION_TIMEOUT)
        .saturating_add(FORCE_TERMINATION_TIMEOUT)
        .saturating_add(FORCE_TERMINATION_CONFIRM_TIMEOUT)
        .saturating_add(PTY_MISSING_EOF_TIMEOUT);
    assert_eq!(MAXIMUM_SCHEDULED_TERMINATION_DRAIN, expected_drain);
    assert!(WORKER_TERMINATION_TIMEOUT > MAXIMUM_SCHEDULED_TERMINATION_DRAIN);
    assert!(WORKER_AUTH_TIMEOUT > WORKER_TERMINATION_TIMEOUT);
    assert_eq!(
        WORKER_TERMINATION_TIMEOUT,
        MAXIMUM_SCHEDULED_TERMINATION_DRAIN.saturating_add(PROCESS_SCAN_DEADLINE_OVERRUN_MARGIN)
    );
    assert_eq!(
        WORKER_AUTH_TIMEOUT,
        WORKER_TERMINATION_TIMEOUT.saturating_add(WORKER_AUTH_RESPONSE_MARGIN)
    );
}

const IMMEDIATE_WORKER_TIMEOUT: Duration = Duration::ZERO;
