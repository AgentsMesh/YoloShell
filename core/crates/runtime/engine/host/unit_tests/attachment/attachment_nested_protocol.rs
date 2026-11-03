use crate::host::tests::*;
use super::*;

const NESTED_PROTOCOL_SOCKET: &str = "nested-protocol.sock";
const WORKER_EOF_SOCKET: &str = "worker-eof.sock";
const NESTED_VERSION_REQUEST_ID: &str = "nested-request-version";
const NESTED_KIND_REQUEST_ID: &str = "nested-request-kind";
const NESTED_UNSUPPORTED_REQUEST_ID: &str = "nested-request-unsupported";
const NESTED_UNSUPPORTED_METHOD: &str = "nested-future-method";
const NESTED_PROTOCOL_VERSION_OFFSET: u16 = 1;

fn nested_request(request_id: &str, method_name: &str) -> RequestEnvelope {
    RequestEnvelope::new(request_id, method_name, yolo_capnp_value::Value::Null)
}

#[tokio::test]
async fn endpoint_validation_remains_active_while_a_separate_observer_capability_exists() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let socket = directory.path().join(NESTED_PROTOCOL_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let worker = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = accept_worker_auth(UnixStream::from(stream), &record).await;
        let request = stream.request().await.unwrap();
        stream
            .respond(
                &ResponseEnvelope::success(&request, &empty_attachment(record.runtime_epoch))
                    .unwrap(),
            )
            .await
            .unwrap();
        stream.request().await
    });
    let (client, server) = UnixStream::pair().unwrap();
    let serving = tokio::spawn(Arc::clone(&host).handle_client(server));
    let client = open_host_rpc(client).await;
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: 0,
        },
    );
    let reply = client
        .request(
            yolo_capnp_rpc::Method::Call,
            yolo_capnp_codec::to_value(&attach).unwrap(),
            tokio::time::Instant::now() + HOST_REQUEST_TIMEOUT,
        )
        .await
        .unwrap();
    let initial: ResponseEnvelope = yolo_capnp_codec::from_value(reply.value).unwrap();
    assert!(initial.ok);
    let events = reply.events.unwrap();

    let mut wrong_version = nested_request(NESTED_VERSION_REQUEST_ID, method::HOST_STATUS);
    wrong_version.version += NESTED_PROTOCOL_VERSION_OFFSET;
    let response = host_rpc(&client, &wrong_version).await;
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    let mut wrong_kind = nested_request(NESTED_KIND_REQUEST_ID, method::HOST_STATUS);
    wrong_kind.kind = crate::protocol::RESPONSE_KIND.into();
    assert!(!host_rpc(&client, &wrong_kind).await.ok);
    let unsupported = nested_request(NESTED_UNSUPPORTED_REQUEST_ID, NESTED_UNSUPPORTED_METHOD);
    assert_eq!(
        host_rpc(&client, &unsupported).await.error.unwrap().code,
        crate::protocol::error_code::UNSUPPORTED_METHOD
    );
    let malformed =
        EventEnvelope::new("not-request", Some(terminal_id.to_string()), None, &()).unwrap();
    assert!(
        client
            .request(
                yolo_capnp_rpc::Method::Call,
                yolo_capnp_codec::to_value(&malformed).unwrap(),
                tokio::time::Instant::now() + HOST_REQUEST_TIMEOUT
            )
            .await
            .is_err()
    );
    assert!(
        host_rpc(&client, &request(method::HOST_STATUS, &()))
            .await
            .ok
    );
    drop(events);
    drop(client);
    let _ = serving.await.unwrap();
    assert!(matches!(
        worker.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}

#[tokio::test]
async fn attachment_reports_worker_eof_after_the_initial_response_as_unavailable() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(WORKER_EOF_SOCKET);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let worker = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = accept_worker_auth(UnixStream::from(stream), &record).await;
        let request = stream.request().await.unwrap();
        stream
            .respond(
                &ResponseEnvelope::success(&request, &empty_attachment(record.runtime_epoch))
                    .unwrap(),
            )
            .await
            .unwrap();
    });
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: 0,
        },
    );
    let (initial, mut events, publication) = host.rpc_attachment(&attach).await.unwrap();
    assert!(initial.ok);
    drop(publication);
    assert!(events.next().await.is_err());
    worker.await.unwrap();
}
