use super::*;

#[path = "../../worker/worker_retirement_observation.rs"]
mod retirement_observation;
#[path = "resume_released_shutdown_drain.rs"]
mod shutdown_drain;

const SOCKET_NAME: &str = "delayed-release.sock";
const SHUTDOWN_OBSERVED: &str = "delayed shutdown was observed";
const LIFECYCLE_RETAINED: &str = "resume retained the lifecycle lock during Worker retirement";

#[tokio::test]
async fn retirement_diagnostics_never_include_the_worker_authentication_token() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let diagnostic = host.unresolved_previous_runtime(&record);
    assert!(
        diagnostic
            .message
            .contains(RESUME_PREVIOUS_RUNTIME_UNRESOLVED)
    );
    assert!(!diagnostic.message.contains(&record.auth_token));
}

#[test]
fn only_an_explicit_shutdown_response_permits_waiting_past_a_remote_rejection() {
    use crate::protocol::error_code;
    for (code, may_settle) in [
        (error_code::WORKER_SHUTTING_DOWN, true),
        (error_code::CONFLICT, false),
        (error_code::IDENTITY_MISMATCH, false),
        (error_code::UNAUTHORIZED, false),
        (error_code::PROTOCOL_INCOMPATIBLE, false),
    ] {
        let error = WorkerRetirementError::control(
            WorkerRetirementControlPhase::ShutdownResponse,
            WorkerRpcError::Remote(ErrorBody::new(code, SHUTDOWN_OBSERVED, true)),
        );
        assert_eq!(error.may_settle_after_disconnect(), may_settle);
    }
}

#[tokio::test]
async fn resume_does_not_block_a_retiring_workers_lifecycle_participant() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let manifest = completed_test_manifest(&host, record.terminal_id);
    persist_host_session(&host, &manifest, &record);
    let paths = host.paths.clone();
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    let authenticated = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let available = paths
            .try_acquire_session_lifecycle_lock(authenticated.terminal_id)
            .unwrap()
            .is_some();
        write_frame(
            &mut stream,
            &ResponseEnvelope::success(&auth, &test_worker_hello(&authenticated)).unwrap(),
        )
        .await
        .unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        let shutdown = stream.request().await.unwrap();
        let detail = if available {
            SHUTDOWN_OBSERVED
        } else {
            LIFECYCLE_RETAINED
        };
        stream
            .respond(&ResponseEnvelope::failure(
                &shutdown,
                ErrorBody::internal(detail),
            ))
            .await
            .unwrap();
        available
    });
    let error = host
        .begin_resume_transaction(manifest, ResumeSelection::explicit(true), None)
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(error.message, SHUTDOWN_OBSERVED);
    assert!(server.await.unwrap());
}

#[tokio::test]
async fn a_live_worker_settlement_probe_does_not_open_unauthenticated_connections() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let listener = UnixListener::bind(&record.socket_path).unwrap();
    assert!(!host.runtime_is_settled(&record, Duration::ZERO).await);
    assert!(
        tokio::time::timeout(PROCESS_EXIT_POLL_INTERVAL, listener.accept())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn resume_observes_a_release_acknowledged_after_settlement_started() {
    let directory = tempfile::tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut record = host_runtime_record(&host, Uuid::new_v4(), directory.path().join(SOCKET_NAME));
    let child = reaped_released_exit_child_identity();
    record.child_pid = child.pid;
    record.child_start_time = child.start_time;
    let mut manifest = completed_test_manifest(&host, record.terminal_id);
    persist_host_session(&host, &manifest, &record);
    atomic_write_json(
        &host.paths.exit_record(record.terminal_id),
        &ExitRecord {
            runtime_epoch: record.runtime_epoch,
            code: Some(EXIT_CODE),
            signal: None,
            reason: ExitReason::Natural,
            observed_at_ms: EXIT_OBSERVED_AT_MS,
            last_output_sequence: EXIT_OUTPUT_SEQUENCE,
        },
    )
    .unwrap();
    let lease = host.secondary_custody.install_test_retired_lease(
        &host,
        &record,
        secondary_custody::TestRetiredCustodyState::ReleaseAckPending,
    );
    let waiting = host.settle_disconnected_previous_runtime(&mut manifest, &record);
    tokio::pin!(waiting);
    assert!(futures::poll!(waiting.as_mut()).is_pending());

    let listener = UnixListener::bind(&record.socket_path).unwrap();
    host.secondary_custody.complete_test_pending_release(&lease);
    let authenticated = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let Ok(auth) = read_frame::<_, RequestEnvelope>(&mut stream).await else {
            return false;
        };
        write_frame(
            &mut stream,
            &ResponseEnvelope::success(&auth, &test_worker_hello(&authenticated)).unwrap(),
        )
        .await
        .unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        let shutdown = stream.request().await.unwrap();
        assert_eq!(shutdown.method, WORKER_SHUTDOWN_METHOD);
        let response = ResponseEnvelope::failure(&shutdown, ErrorBody::internal(SHUTDOWN_OBSERVED));
        stream.respond(&response).await.unwrap();
        true
    });
    let error = waiting.await.unwrap_err().into_explicit_error();
    assert_eq!(error.message, SHUTDOWN_OBSERVED);
    assert!(server.await.unwrap());
}
