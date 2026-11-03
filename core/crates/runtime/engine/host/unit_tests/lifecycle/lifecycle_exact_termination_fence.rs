use crate::host::tests::*;
const LEGACY_WORKER_UNSUPPORTED_DETAIL: &str = "legacy worker rejects exact termination";
const EXACT_STATUS_CHANGED_DETAIL: &str = "runtime status changed before exact termination";
const EXACT_TERMINATION_SOCKET_NAME: &str = "exact-termination.sock";
const EXACT_REPLACEMENT_SOCKET_NAME: &str = "exact-replacement.sock";

#[tokio::test]
async fn legacy_worker_cannot_downgrade_exact_termination() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let socket = directory.path().join(EXACT_TERMINATION_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let runtime = host_runtime_record(&host, manifest.terminal_id, socket);
    persist_host_session(&host, &manifest, &runtime);
    let server_runtime = runtime.clone();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept_exact_request(listener, &server_runtime).await;
        assert_eq!(request.method, method::TERMINATE_RUNTIME_EXACT);
        let payload: crate::worker::FencedTerminateRequest = request.decode_payload().unwrap();
        assert_eq!(payload.runtime_epoch, server_runtime.runtime_epoch);
        let response = ResponseEnvelope::failure(
            &request,
            ErrorBody::new(
                crate::protocol::error_code::UNSUPPORTED_METHOD,
                LEGACY_WORKER_UNSUPPORTED_DETAIL,
                false,
            ),
        );
        stream.respond(&response).await.unwrap();
    });

    let error = host
        .terminate_product_request(TerminalTerminateRequest {
            selection: TerminalLifecycleSelection::unconditional(manifest.terminal_id),
            force: true,
        })
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::UNSUPPORTED_METHOD);
    assert!(
        !host
            .paths
            .terminate_tombstone(manifest.terminal_id)
            .exists()
    );
    await_exact_termination_worker(server).await;
}

#[tokio::test]
async fn exact_termination_rejects_status_change_at_worker_boundary() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let socket = directory.path().join(EXACT_TERMINATION_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let runtime = host_runtime_record(&host, manifest.terminal_id, socket);
    persist_host_session(&host, &manifest, &runtime);
    let server_runtime = runtime.clone();
    let server = tokio::spawn(async move {
        let mut stream = serve_selected_unreachable_status(listener, &server_runtime).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        assert_eq!(request.method, method::TERMINATE_RUNTIME_EXACT);
        let payload: crate::worker::FencedTerminateRequest = request.decode_payload().unwrap();
        assert_eq!(
            payload.expected_status,
            Some(TerminalRuntimeStatus::Unreachable)
        );
        let response =
            ResponseEnvelope::failure(&request, ErrorBody::conflict(EXACT_STATUS_CHANGED_DETAIL));
        stream.respond(&response).await.unwrap();
    });
    let selection = TerminalLifecycleSelection::selected(
        manifest.terminal_id,
        manifest.updated_at_ms,
        TerminalRuntimeStatus::Unreachable,
    )
    .with_expected_runtime_epoch(Some(runtime.runtime_epoch));

    let error = host
        .terminate_product_request(TerminalTerminateRequest {
            selection,
            force: false,
        })
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(
        !host
            .paths
            .terminate_tombstone(manifest.terminal_id)
            .exists()
    );
    await_exact_termination_worker(server).await;
}

#[tokio::test]
async fn exact_termination_success_cannot_authorize_a_successor() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let socket = directory.path().join(EXACT_TERMINATION_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let runtime = host_runtime_record(&host, manifest.terminal_id, socket);
    persist_host_session(&host, &manifest, &runtime);
    let replacement_socket = directory.path().join(EXACT_REPLACEMENT_SOCKET_NAME);
    let replacement_listener = UnixListener::bind(&replacement_socket).unwrap();
    let mut replacement = runtime.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    replacement.worker_id = Uuid::new_v4();
    replacement.socket_path = replacement_socket;
    let expected = replacement.clone();
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    let server_runtime = runtime.clone();
    let server = tokio::spawn(async move {
        let (mut stream, request) = accept_exact_request(listener, &server_runtime).await;
        atomic_write_json(&runtime_path, &replacement).unwrap();
        stream
            .respond(&ResponseEnvelope::empty(&request))
            .await
            .unwrap();
    });

    let error = host
        .terminate_product_request(TerminalTerminateRequest {
            selection: TerminalLifecycleSelection::at_runtime_epoch(
                manifest.terminal_id,
                runtime.runtime_epoch,
            ),
            force: true,
        })
        .await
        .unwrap_err();

    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id)).unwrap(),
        expected
    );
    assert_listener_received_no_connection(replacement_listener);
    await_exact_termination_worker(server).await;
}

async fn accept_exact_request(
    listener: UnixListener,
    runtime: &RuntimeRecord,
) -> (yolo_runtime_rpc_testing::Peer, RequestEnvelope) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, runtime).await;
    let request = stream.request().await.unwrap();
    (stream, request)
}

async fn serve_selected_unreachable_status(
    listener: UnixListener,
    runtime: &RuntimeRecord,
) -> yolo_runtime_rpc_testing::Peer {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, runtime).await;
    let request: RequestEnvelope = stream.request().await.unwrap();
    assert_eq!(request.method, WORKER_STATUS_METHOD);
    let mut status = status_view(runtime, Some(Vec::new()));
    status.status = TerminalRuntimeStatus::Unreachable;
    stream
        .respond(&ResponseEnvelope::success(&request, &status).unwrap())
        .await
        .unwrap();
    stream
}

async fn await_exact_termination_worker(server: tokio::task::JoinHandle<()>) {
    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, server)
        .await
        .unwrap()
        .unwrap();
}
