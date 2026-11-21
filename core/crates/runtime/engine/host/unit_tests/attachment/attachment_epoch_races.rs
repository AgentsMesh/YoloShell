use crate::host::tests::*;
use super::*;

use crate::model::OutputChunk;
use crate::protocol::{EventEnvelope, ProtocolError, event};

const REPLACEMENT_SOCKET_NAME: &str = "replacement-worker.sock";
const REPLACEMENT_AUTH_TOKEN: &str = "replacement-worker-token";
const DELAYED_WORKER_SOCKET_NAME: &str = "delayed-old-worker.sock";
const OLD_EVENT_WORKER_SOCKET_NAME: &str = "old-event-worker.sock";
const CONTROLLER_ADOPTION_SOCKET_NAME: &str = "controller-adoption-worker.sock";
pub(super) const RACE_ATTACH_AFTER_SEQUENCE: u64 = 0;
const OLD_EVENT_SEQUENCE: u64 = 1;

#[tokio::test]
async fn attachment_epoch_host_rejects_delayed_old_runtime_after_publication() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(DELAYED_WORKER_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, selected) = attachment_record(&host, socket);
    let server_record = selected.clone();
    let (requested_tx, requested_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &server_record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        requested_tx.send(()).unwrap();
        release_rx.await.unwrap();
        let initial = empty_attachment(server_record.runtime_epoch);
        let response = ResponseEnvelope::success(&request, &initial).unwrap();
        stream.respond(&response).await.unwrap();
    });
    let mut replacement = selected.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    replacement.worker_id = Uuid::new_v4();
    replacement.socket_path = directory.path().join(REPLACEMENT_SOCKET_NAME);
    replacement.auth_token = REPLACEMENT_AUTH_TOKEN.into();
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: RACE_ATTACH_AFTER_SEQUENCE,
        },
    );
    let observer = async {
        requested_rx.await.unwrap();
        atomic_write_json(&host.paths.runtime_record(terminal_id), &replacement).unwrap();
        release_tx.send(()).unwrap();
    };
    let (result, ()) = tokio::join!(host.rpc_attachment(&attach), observer);
    let error = result.err().unwrap();
    assert!(!error.retryable);
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    server.await.unwrap();
}

#[tokio::test]
async fn attachment_epoch_host_never_forwards_old_buffered_event() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(OLD_EVENT_WORKER_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let expected_epoch = record.runtime_epoch;
    let old_epoch = Uuid::new_v4();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        let initial = empty_attachment(expected_epoch);
        stream
            .respond(&ResponseEnvelope::success(&request, &initial).unwrap())
            .await
            .unwrap();
        let chunk = OutputChunk {
            runtime_epoch: old_epoch,
            sequence: OLD_EVENT_SEQUENCE,
            data: Vec::new(),
        };
        let event = EventEnvelope::new(
            event::TERMINAL_OUTPUT,
            Some(terminal_id.to_string()),
            Some(old_epoch.to_string()),
            &chunk,
        )
        .unwrap();
        stream.event(&event).await.unwrap();
        let _ = stream.request().await;
    });
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: RACE_ATTACH_AFTER_SEQUENCE,
        },
    );
    let (initial, mut events, publication) = host.rpc_attachment(&attach).await.unwrap();
    assert!(initial.ok);
    drop(publication);
    assert!(events.next().await.is_err());
    drop(events);
    server.await.unwrap();
}

#[tokio::test]
async fn attachment_epoch_host_allows_same_identity_controller_adoption() {
    let directory = tempdir().unwrap();
    let previous_host = Host::open(HostConfig::new(directory.path())).unwrap();
    let previous_generation = previous_host.controller_generation;
    drop(previous_host);
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    assert!(host.controller_generation > previous_generation);
    let socket = directory.path().join(CONTROLLER_ADOPTION_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, mut selected) = attachment_record(&host, socket);
    selected.controller_generation = previous_generation;
    atomic_write_json(&host.paths.runtime_record(terminal_id), &selected).unwrap();
    let current_generation = host.controller_generation;
    let runtime_path = host.paths.runtime_record(terminal_id);
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let auth: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let mut adopted = selected;
        adopted.controller_generation = current_generation;
        atomic_write_json(&runtime_path, &adopted).unwrap();
        write_frame(
            &mut stream,
            &ResponseEnvelope::success(&auth, &test_worker_hello(&adopted)).unwrap(),
        )
        .await
        .unwrap();
        let mut stream = yolo_runtime_rpc_testing::Peer::new(stream);
        let attach: RequestEnvelope = stream.request().await.unwrap();
        let initial = empty_attachment(adopted.runtime_epoch);
        stream
            .respond(&ResponseEnvelope::success(&attach, &initial).unwrap())
            .await
            .unwrap();
        stream.request().await
    });
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: RACE_ATTACH_AFTER_SEQUENCE,
        },
    );
    let (initial, events, publication) = host.rpc_attachment(&attach).await.unwrap();
    assert!(initial.ok);
    drop(publication);
    drop(events);
    assert!(matches!(
        server.await.unwrap(),
        Err(ProtocolError::EndOfStream)
    ));
}
