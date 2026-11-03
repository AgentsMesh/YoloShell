use crate::host::tests::*;
use super::*;

const STATUS_REUSE_SOCKET_NAME: &str = "status-reuse.sock";
const STATUS_REUSE_OUTPUT_SEQUENCE: u64 = 0;

#[tokio::test]
async fn worker_rpc_failure_fallbacks_are_stable() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let status_socket = directory.path().join("status.sock");
    let listener = UnixListener::bind(&status_socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), status_socket);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &server_record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        assert!(matches!(
            stream.respond(&response_without_error(&request)).await,
            Err(ProtocolError::EndOfStream)
        ));
    });
    let error = host.worker_status(&record).await.unwrap_err();
    assert!(matches!(
        error,
        WorkerRpcError::Protocol(detail)
            if detail == crate::protocol::ProtocolError::ResponseMismatch.to_string()
    ));
    server.await.unwrap();

    let retire_socket = directory.path().join("retire.sock");
    let listener = UnixListener::bind(&retire_socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), retire_socket);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &server_record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        assert!(matches!(
            stream.respond(&response_without_error(&request)).await,
            Err(ProtocolError::EndOfStream)
        ));
    });
    let error = host.retire_worker(&record).await.unwrap_err();
    assert_eq!(
        error,
        ErrorBody::protocol_incompatible(
            crate::protocol::ProtocolError::ResponseMismatch.to_string()
        )
    );
    server.await.unwrap();
}

#[tokio::test]
async fn worker_status_reuses_one_authenticated_stream() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(STATUS_REUSE_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), socket);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let hello = crate::model::WorkerHello {
            runtime: server_record.clone(),
            capabilities: Vec::new(),
        };
        write_frame(
            &mut stream,
            &ResponseEnvelope::success(&auth, &hello).unwrap(),
        )
        .await
        .unwrap();
        let status = RuntimeView {
            presentation: Default::default(),
            runtime_epoch: server_record.runtime_epoch,
            worker_id: server_record.worker_id,
            worker_pid: server_record.worker_pid,
            worker_start_time: server_record.worker_start_time,
            child_pid: server_record.child_pid,
            child_start_time: server_record.child_start_time,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: STATUS_REUSE_OUTPUT_SEQUENCE,
            exit: None,
            bootstrap_delivery: server_record.bootstrap_delivery,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        };
        let mut peer = yolo_runtime_rpc_testing::Peer::new(stream);
        for _ in 0..2 {
            let request = peer.request().await.unwrap();
            peer.respond(&ResponseEnvelope::success(&request, &status).unwrap())
                .await
                .unwrap();
        }
    });
    assert_eq!(
        host.worker_status(&record).await.unwrap().runtime_epoch,
        record.runtime_epoch
    );
    assert_eq!(
        host.worker_status(&record).await.unwrap().runtime_epoch,
        record.runtime_epoch
    );
    server.await.unwrap();
}

#[tokio::test]
async fn proxied_worker_responses_keep_client_correlation() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    host.paths.ensure_session(terminal_id).unwrap();
    let socket = directory.path().join("proxy.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, terminal_id, socket);
    atomic_write_json(&host.paths.runtime_record(terminal_id), &record).unwrap();
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &server_record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        let response = ResponseEnvelope::success(&request, &"worker").unwrap();
        stream.respond(&response).await.unwrap();
    });
    let request = request(
        method::GET_SNAPSHOT,
        &crate::worker::SnapshotRequest {
            terminal_id,
            runtime_epoch: record.runtime_epoch,
        },
    );
    let response = host
        .proxy_runtime_bound_worker_request(&request)
        .await
        .unwrap();
    assert_eq!(response.request_id, "test-request");
    assert_eq!(response.method, method::GET_SNAPSHOT);
    server.await.unwrap();
}

#[test]
fn bootstrap_outcomes_gate_host_readiness_and_runtime_access() {
    assert!(validate_runtime_bootstrap(BootstrapDelivery::NotRequired).is_ok());
    assert!(validate_runtime_bootstrap(BootstrapDelivery::Delivered).is_ok());
    let pending = validate_runtime_bootstrap(BootstrapDelivery::Pending).unwrap_err();
    assert_eq!(pending.code, crate::protocol::error_code::DELIVERY_UNKNOWN);
    let failed = validate_runtime_bootstrap(BootstrapDelivery::Failed).unwrap_err();
    assert_eq!(
        failed.code,
        crate::protocol::error_code::WORKER_START_FAILED
    );
    let unknown = validate_runtime_bootstrap(BootstrapDelivery::Unknown).unwrap_err();
    assert_eq!(unknown.code, crate::protocol::error_code::DELIVERY_UNKNOWN);

    assert!(matches!(
        bootstrap_worker_start(BootstrapDelivery::NotRequired).unwrap(),
        BootstrapWorkerAction::Authenticate
    ));
    assert!(matches!(
        bootstrap_worker_start(BootstrapDelivery::Delivered).unwrap(),
        BootstrapWorkerAction::Authenticate
    ));
    assert!(matches!(
        bootstrap_worker_start(BootstrapDelivery::Pending).unwrap(),
        BootstrapWorkerAction::AwaitFinal
    ));
    assert!(bootstrap_worker_start(BootstrapDelivery::Failed).is_err());
    assert!(matches!(
        bootstrap_worker_start(BootstrapDelivery::Unknown).unwrap(),
        BootstrapWorkerAction::Uncertain(_)
    ));
}

include!("../secondary_custody/secondary_custody_rpc_test_authority_tests.rs");
