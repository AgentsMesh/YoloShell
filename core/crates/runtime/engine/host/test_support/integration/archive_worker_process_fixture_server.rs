use crate::host::tests::*;
const ARCHIVE_WORKER_FIXTURE_UNKNOWN_MODE: &str = "unknown archive worker fixture mode";

async fn run_archive_worker_server(
    root: &Path,
    socket: &Path,
    record: &RuntimeRecord,
    mode: &str,
    child: &mut std::process::Child,
) {
    let listener = bind_archive_fixture_listener(socket);
    if matches!(
        mode,
        ARCHIVE_WORKER_EXITED_SHUTDOWN_REPLACEMENT_MODE
            | ARCHIVE_WORKER_RELEASED_SHUTDOWN_EOF_MODE
            | ARCHIVE_WORKER_REJECTED_SHUTDOWN_EOF_MODE
    ) {
        stop_resume_test_child(child);
    }
    std::fs::write(
        root.join(ARCHIVE_WORKER_READY_FILE),
        ARCHIVE_WORKER_FIXTURE_MARKER,
    )
    .unwrap();
    match mode {
        ARCHIVE_WORKER_RELEASED_SHUTDOWN_EOF_MODE | ARCHIVE_WORKER_REJECTED_SHUTDOWN_EOF_MODE => {
            serve_released_fixture_shutdown(&listener, record, mode).await;
        }
        ARCHIVE_WORKER_READY_HOLD_MODE => serve_archive_fixture_ready(&listener, record).await,
        ARCHIVE_WORKER_STATUS_DISCONNECT_BOTH_MODE => {
            serve_archive_fixture_status_disconnect(&listener, record).await;
            stop_resume_test_child(child);
        }
        ARCHIVE_WORKER_EXITED_SHUTDOWN_DROP_BOTH_MODE => {
            let mut stream = serve_archive_fixture_exited(&listener, record).await;
            let request = stream.request().await.unwrap();
            assert_eq!(request.method, WORKER_SHUTDOWN_METHOD);
            stream.close().await;
            stop_resume_test_child(child);
        }
        ARCHIVE_WORKER_EXITED_EXIT_BOTH_MODE => {
            serve_archive_fixture_exited(&listener, record)
                .await
                .close()
                .await;
            stop_resume_test_child(child);
        }
        ARCHIVE_WORKER_EXITED_EXIT_WORKER_MODE => {
            serve_archive_fixture_exited(&listener, record)
                .await
                .close()
                .await;
        }
        ARCHIVE_WORKER_EXITED_SHUTDOWN_REPLACEMENT_MODE => {
            let mut stream = serve_archive_fixture_exited(&listener, record).await;
            let request = stream.request().await.unwrap();
            assert_eq!(request.method, WORKER_SHUTDOWN_METHOD);
            drop(listener);
            std::fs::remove_file(socket).unwrap();
            spawn_archive_replacement(root, socket);
            stream
                .respond(&ResponseEnvelope::empty(&request))
                .await
                .unwrap();
            // This fixture is a process: keep its RPC executor alive until the
            // client has consumed the confirmation and released the channel.
            assert!(matches!(
                stream.request().await,
                Err(crate::protocol::ProtocolError::EndOfStream)
            ));
            return;
        }
        _ => panic!("{ARCHIVE_WORKER_FIXTURE_UNKNOWN_MODE}"),
    }
    drop(listener);
    let _ = std::fs::remove_file(socket);
}

async fn serve_released_fixture_shutdown(
    listener: &UnixListener,
    record: &RuntimeRecord,
    mode: &str,
) {
    let (mut stream, request) = accept_archive_fixture_shutdown(listener, record).await;
    let response = if mode == ARCHIVE_WORKER_REJECTED_SHUTDOWN_EOF_MODE {
        ResponseEnvelope::failure(
            &request,
            ErrorBody::new(
                crate::protocol::error_code::IDENTITY_MISMATCH,
                ARCHIVE_WORKER_RELEASED_SHUTDOWN_REJECTED,
                false,
            ),
        )
    } else {
        ResponseEnvelope::empty(&request)
    };
    stream.respond(&response).await.unwrap();
    assert!(matches!(
        stream.request().await,
        Err(crate::protocol::ProtocolError::EndOfStream)
    ));
}

fn bind_archive_fixture_listener(socket: &Path) -> UnixListener {
    use std::os::unix::fs::PermissionsExt;
    let listener = UnixListener::bind(socket).unwrap();
    std::fs::set_permissions(
        socket,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    listener
}

async fn serve_archive_fixture_ready(listener: &UnixListener, record: &RuntimeRecord) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, record).await;
    drop(stream);
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, record).await;
    let request: RequestEnvelope = stream.request().await.unwrap();
    let mut view = archive_exited_view(record, archive_exit_for(record.runtime_epoch));
    view.status = TerminalRuntimeStatus::Running;
    view.exit = None;
    stream
        .respond(&ResponseEnvelope::success(&request, &view).unwrap())
        .await
        .unwrap();
    std::future::pending::<()>().await;
}

async fn serve_archive_fixture_status_disconnect(listener: &UnixListener, record: &RuntimeRecord) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, record).await;
    let _: RequestEnvelope = stream.request().await.unwrap();
}

async fn serve_archive_fixture_exited(
    listener: &UnixListener,
    record: &RuntimeRecord,
) -> yolo_runtime_rpc_testing::Peer {
    let (mut stream, request) = accept_archive_request(listener, record).await;
    assert_eq!(request.method, WORKER_STATUS_METHOD);
    let view = archive_exited_view(record, archive_exit_for(record.runtime_epoch));
    stream
        .respond(&ResponseEnvelope::success(&request, &view).unwrap())
        .await
        .unwrap();
    // respond waits for the client to observe the reply. Ordinary worker RPCs
    // reuse this authenticated stream, so the next request may be shutdown.
    stream
}

async fn accept_archive_fixture_shutdown(
    listener: &UnixListener,
    record: &RuntimeRecord,
) -> (yolo_runtime_rpc_testing::Peer, RequestEnvelope) {
    let (stream, request) = accept_archive_request(listener, record).await;
    assert_eq!(request.method, WORKER_SHUTDOWN_METHOD);
    (stream, request)
}

fn spawn_archive_replacement(root: &Path, socket: &Path) {
    std::fs::write(
        root.join(ARCHIVE_REPLACEMENT_REQUEST_FILE),
        ARCHIVE_WORKER_FIXTURE_MARKER,
    )
    .unwrap();
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command
        .args([
            crate::constants::TEST_HARNESS_EXACT_ARGUMENT,
            ARCHIVE_WORKER_FIXTURE_TEST,
            crate::constants::TEST_HARNESS_NOCAPTURE_ARGUMENT,
            crate::constants::TEST_HARNESS_SINGLE_THREAD_ARGUMENT,
        ])
        .env(
            ARCHIVE_WORKER_FIXTURE_ROLE_ENV,
            ARCHIVE_WORKER_REPLACEMENT_ROLE,
        )
        .env(ARCHIVE_WORKER_FIXTURE_ROOT_ENV, root)
        .env(ARCHIVE_WORKER_FIXTURE_SOCKET_ENV, socket)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let _replacement = crate::process::spawn_waitable_test_child(command);
    wait_archive_fixture_path(&root.join(ARCHIVE_REPLACEMENT_READY_FILE));
}
