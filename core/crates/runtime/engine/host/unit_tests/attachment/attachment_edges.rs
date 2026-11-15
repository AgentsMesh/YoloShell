use crate::host::tests::*;
use super::*;

pub(super) const EMPTY_ATTACHMENT_NEXT_SEQUENCE: u64 = 1;

pub(super) fn attachment_record(host: &Host, socket: PathBuf) -> (Uuid, RuntimeRecord) {
    let terminal_id = Uuid::new_v4();
    host.paths.ensure_session(terminal_id).unwrap();
    let record = host_runtime_record(host, terminal_id, socket);
    atomic_write_json(&host.paths.runtime_record(terminal_id), &record).unwrap();
    (terminal_id, record)
}

pub(super) fn empty_attachment(runtime_epoch: Uuid) -> crate::worker::AttachResult {
    crate::worker::AttachResult {
        runtime_epoch,
        snapshot: None,
        chunks: Vec::new(),
        next_sequence: EMPTY_ATTACHMENT_NEXT_SEQUENCE,
        resynchronized: false,
    }
}

#[tokio::test]
async fn attachment_stops_when_worker_rejects_initial_request() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join("reject-attach.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = accept_worker_auth(UnixStream::from(stream), &record).await;
        let request = stream.request().await.unwrap();
        stream
            .respond(&ResponseEnvelope::failure(
                &request,
                ErrorBody::conflict("rejected"),
            ))
            .await
            .unwrap();
    });
    let request = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: 0,
        },
    );
    let error = host.rpc_attachment(&request).await.err().unwrap();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    server.await.unwrap();
}

#[tokio::test]
async fn malformed_worker_event_terminates_attachment_with_protocol_error() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join("bad-event.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let server = tokio::spawn(async move {
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
        let invalid = RequestEnvelope::new(
            "not-event",
            method::HOST_STATUS,
            yolo_capnp_value::Value::Null,
        );
        stream
            .raw_event(yolo_capnp_codec::to_value(&invalid).unwrap())
            .await
            .unwrap();
        assert!(stream.request().await.is_err());
    });
    let request = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: 0,
        },
    );
    let (response, mut events, publication) = host.rpc_attachment(&request).await.unwrap();
    assert!(response.ok);
    drop(publication);
    assert!(events.next().await.is_err());
    drop(events);
    server.await.unwrap();
}

#[tokio::test]
async fn dropping_observer_capability_closes_the_worker_subscription_without_a_terminal_command() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join("disconnect.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let server = tokio::spawn(async move {
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
    let request = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: 0,
        },
    );
    let (response, events, publication) = host.rpc_attachment(&request).await.unwrap();
    assert!(response.ok);
    drop(publication);
    drop(events);
    assert!(matches!(
        server.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}
