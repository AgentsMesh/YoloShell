use crate::host::tests::*;
use super::*;

const STATUS_CAPABILITY_SOCKET_NAME: &str = "status-capability.sock";
const STATUS_OUTPUT_SEQUENCE: u64 = 0;

#[tokio::test]
async fn worker_status_uses_authenticated_runtime_capabilities() {
    assert_current_status_capabilities(vec![
        crate::protocol::capability::SAFE_PROCESS_TREE_TERMINATION.to_owned(),
    ])
    .await;
}

#[tokio::test]
async fn worker_status_preserves_current_unavailable_capability() {
    assert_current_status_capabilities(Vec::new()).await;
}

#[tokio::test]
async fn worker_status_rejects_auth_without_a_capability_declaration() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(STATUS_CAPABILITY_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), socket);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        write_frame(
            &mut stream,
            &ResponseEnvelope::success(&auth, &server_record).unwrap(),
        )
        .await
        .unwrap();
    });

    assert!(host.worker_status(&record).await.is_err());
    server.await.unwrap();
}

async fn assert_current_status_capabilities(capabilities: Vec<String>) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(STATUS_CAPABILITY_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), socket);
    let status = status_view(&record, Some(capabilities.clone()));
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        serve_capability_status(listener, server_record, status).await;
    });

    let result = host.worker_status(&record).await.unwrap();
    assert_eq!(result.capabilities, Some(capabilities));
    server.await.unwrap();
}

#[tokio::test]
async fn worker_status_rejects_capability_mismatch() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(STATUS_CAPABILITY_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), socket);
    let status = status_view(&record, Some(Vec::new()));
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let hello = crate::model::WorkerHello {
            runtime: server_record,
            capabilities: vec![
                crate::protocol::capability::SAFE_PROCESS_TREE_TERMINATION.to_owned(),
            ],
        };
        write_frame(
            &mut stream,
            &ResponseEnvelope::success(&auth, &hello).unwrap(),
        )
        .await
        .unwrap();
        reply_status(stream, status).await;
    });

    let error = host.worker_status(&record).await.unwrap_err();
    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    server.await.unwrap();
}

#[tokio::test]
async fn worker_status_rejects_unknown_and_known_empty_provenance_mismatch() {
    assert_capability_provenance_mismatch(Vec::new(), None).await;
}

async fn assert_capability_provenance_mismatch(
    authenticated: Vec<String>,
    reported: Option<Vec<String>>,
) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(STATUS_CAPABILITY_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), socket);
    let status = status_view(&record, reported);
    let server_record = record.clone();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let request: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let response = ResponseEnvelope::success(
            &request,
            &crate::model::WorkerHello {
                runtime: server_record,
                capabilities: authenticated,
            },
        )
        .unwrap();
        write_frame(&mut stream, &response).await.unwrap();
        reply_status(stream, status).await;
    });

    let error = host.worker_status(&record).await.unwrap_err();
    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    server.await.unwrap();
}

pub(super) async fn serve_capability_status(
    listener: UnixListener,
    record: RuntimeRecord,
    status: RuntimeView,
) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
    let hello = crate::model::WorkerHello {
        runtime: record,
        capabilities: status.capabilities.clone().unwrap(),
    };
    let response = ResponseEnvelope::success(&auth, &hello).unwrap();
    write_frame(&mut stream, &response).await.unwrap();
    reply_status(stream, status).await;
}

async fn reply_status(stream: UnixStream, status: RuntimeView) {
    let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
    let request = stream.request().await.unwrap();
    let response = ResponseEnvelope::success(&request, &status).unwrap();
    stream.respond(&response).await.unwrap();
}

pub(super) fn status_view(record: &RuntimeRecord, capabilities: Option<Vec<String>>) -> RuntimeView {
    RuntimeView {
        presentation: Default::default(),
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        worker_pid: record.worker_pid,
        worker_start_time: record.worker_start_time,
        child_pid: record.child_pid,
        child_start_time: record.child_start_time,
        status: TerminalRuntimeStatus::Running,
        last_output_sequence: STATUS_OUTPUT_SEQUENCE,
        exit: None,
        bootstrap_delivery: record.bootstrap_delivery,
        capabilities,
        fault_reap: None,
    }
}
