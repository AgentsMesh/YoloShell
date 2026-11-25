use crate::worker::tests::*;
#[cfg(target_os = "macos")]
const RESPONSE_DRAIN_SERVICE_SOCKET: &str = "response-drain-runtime.sock";
#[cfg(target_os = "macos")]
const SHUTDOWN_LISTENER_DETAIL: &str =
    "worker listener accepts connections during shutdown cleanup";

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn runtime_shutdown_closes_the_listener_before_async_cleanup() {
    let fixture = Fixture::new();
    fixture.state.fence_all_custody().await;
    let (custody, mut custody_peer, offer) = pending_activation_lease(&fixture);
    let record = lock_recover(&fixture.state.record).clone();
    let path = fixture
        ._filesystem
        .scratch_path(RESPONSE_DRAIN_SERVICE_SOCKET);
    let listener = UnixListener::bind(&path).unwrap();
    let shell = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    let runtime = tokio::spawn(serve_worker_runtime(
        listener,
        Arc::clone(&fixture.state),
        None,
        shell,
        custody,
        JoinSet::new(),
    ));
    receive_exact_worker_activation(&mut custody_peer, &offer, record.controller_generation).await;
    fixture
        .state
        .control_test_hooks
        .runtime_before_cleanup
        .arm();
    commit_worker_shutdown_for_response_drain(&fixture.state).await;
    fixture
        .state
        .control_test_hooks
        .runtime_before_cleanup
        .wait_until_reached()
        .await;

    assert!(!runtime.is_finished());
    assert!(path.exists());
    let connection = UnixStream::connect(&path).await;
    assert!(
        matches!(connection, Err(error) if error.kind() == io::ErrorKind::ConnectionRefused),
        "{SHUTDOWN_LISTENER_DETAIL}"
    );
    fixture
        .state
        .control_test_hooks
        .runtime_before_cleanup
        .resume();
    runtime.await.unwrap().unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test(flavor = "current_thread", start_paused = true)]
async fn response_drain_runtime_aborts_a_permanently_blocked_write_at_deadline() {
    let fixture = Fixture::new();
    fixture.state.fence_all_custody().await;
    let (custody, mut custody_peer, offer) = pending_activation_lease(&fixture);
    let record = lock_recover(&fixture.state.record).clone();
    let listener_path = fixture
        ._filesystem
        .scratch_path(RESPONSE_DRAIN_SERVICE_SOCKET);
    let listener = UnixListener::bind(&listener_path).unwrap();
    let state = Arc::clone(&fixture.state);
    let generation = record.controller_generation;
    let shell = process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    };
    let runtime = tokio::spawn(serve_worker_runtime(
        listener,
        state,
        None,
        shell,
        custody,
        JoinSet::new(),
    ));
    receive_exact_worker_activation(&mut custody_peer, &offer, generation).await;

    let mut client = UnixStream::connect(listener_path).await.unwrap();
    authenticate_response_drain_service(&mut client, generation).await;
    let deadline = tokio::time::Instant::now() + crate::session_timing::WORKER_READY_TIMEOUT
        + crate::session_timing::WORKER_READY_TIMEOUT;
    let client = yolo_capnp_rpc::Connection::open(client, yolo_capnp_rpc::Protocol::Endpoint, deadline).await.unwrap();
    fixture.state.control_test_hooks.response_before_write.arm();
    let pending = tokio::spawn(async move {
        client.request(yolo_capnp_rpc::Method::Call, yolo_capnp_codec::to_value(&status_request()).unwrap(), deadline).await
    });
    drive_rpc(fixture
        .state
        .control_test_hooks
        .response_before_write
        .wait_until_reached())
        .await;
    commit_worker_shutdown_for_response_drain(&fixture.state).await;

    runtime.await.unwrap().unwrap();
    assert!(drive_rpc(fixture.state.wait_for_response_drain(RESPONSE_DRAIN_CONNECTION_TIMEOUT)).await);
    assert!(drive_rpc(pending).await.unwrap().is_err());
}

#[cfg(target_os = "macos")]
async fn authenticate_response_drain_service(client: &mut UnixStream, generation: u64) {
    let request = RequestEnvelope::new(
        RESPONSE_DRAIN_AUTH_REQUEST_ID,
        WORKER_AUTH_METHOD,
        to_value(&WorkerAuth {
            token: FIXTURE_AUTH_TOKEN.into(),
            controller_generation: generation,
        })
        .unwrap(),
    );
    write_frame(client, &request).await.unwrap();
    let response: ResponseEnvelope = read_frame(client).await.unwrap();
    assert!(response.ok);
}
