use crate::host::tests::*;
use super::*;

const OLD_CONTROL_SOCKET_NAME: &str = "old-control-worker.sock";
const NEW_CONTROL_SOCKET_NAME: &str = "new-control-worker.sock";
const NEW_CONTROL_AUTH_TOKEN: &str = "new-control-worker-token";
const MALFORMED_CONTROL_SOCKET_NAME: &str = "malformed-control-worker.sock";
const MALFORMED_UUID_TEXT: &str = "not-a-uuid";
const CONTROL_AFTER_SEQUENCE: u64 = 0;
const CONTROL_GENERATION: u64 = 1;
const NO_WORKER_CONNECTION_TIMEOUT: Duration = Duration::from_millis(100);
const MALFORMED_CONTROL_REQUEST_ID: &str = "malformed-control-target";

#[tokio::test]
async fn control_epoch_old_attachment_cannot_mutate_replacement_runtime() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let old_socket = directory.path().join(OLD_CONTROL_SOCKET_NAME);
    let old_listener = UnixListener::bind(&old_socket).unwrap();
    let (terminal_id, old_record) = attachment_record(&host, old_socket);
    let old_server_record = old_record.clone();
    let old_server = tokio::spawn(async move {
        let (stream, _) = old_listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &old_server_record).await;
        let attach: RequestEnvelope = stream.request().await.unwrap();
        stream
            .respond(
                &ResponseEnvelope::success(
                    &attach,
                    &empty_attachment(old_server_record.runtime_epoch),
                )
                .unwrap(),
            )
            .await
            .unwrap();
        stream.request().await
    });
    let new_socket = directory.path().join(NEW_CONTROL_SOCKET_NAME);
    let new_listener = UnixListener::bind(&new_socket).unwrap();
    let mut replacement = old_record.clone();
    replacement.runtime_epoch = Uuid::new_v4();
    replacement.worker_id = Uuid::new_v4();
    replacement.socket_path = new_socket;
    replacement.auth_token = NEW_CONTROL_AUTH_TOKEN.into();
    let attach = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: CONTROL_AFTER_SEQUENCE,
        },
    );
    let (initial, events, publication) = host.rpc_attachment(&attach).await.unwrap();
    assert!(initial.ok);
    drop(publication);
    atomic_write_json(&host.paths.runtime_record(terminal_id), &replacement).unwrap();
    let writer_capability = crate::model::WriterCapability::generate();
    let acquire = crate::lease::LeaseRequest {
        terminal_id,
        runtime_epoch: old_record.runtime_epoch,
        holder_id: "stale-controller".into(),
        ttl_ms: None,
        generation: None,
        controller_instance_id: None,
        writer_capability: writer_capability.clone(),
    };
    assert_epoch_mismatch(
        host.dispatch(&request(method::ACQUIRE_WRITER, &acquire))
            .await,
    );
    let input = crate::worker::InputRequest {
        terminal_id,
        runtime_epoch: old_record.runtime_epoch,
        generation: CONTROL_GENERATION,
        writer_capability,
        input_id: Uuid::new_v4(),
        data: b"must-not-reach-new-pty".to_vec(),
    };
    assert_epoch_mismatch(
        host.dispatch(&request(method::TERMINAL_INPUT, &input))
            .await,
    );
    assert!(
        tokio::time::timeout(NO_WORKER_CONNECTION_TIMEOUT, new_listener.accept())
            .await
            .is_err()
    );
    drop(events);
    assert!(matches!(
        old_server.await.unwrap(),
        Err(crate::protocol::ProtocolError::EndOfStream)
    ));
}

fn assert_epoch_mismatch(response: ResponseEnvelope) {
    assert!(!response.ok);
    assert_eq!(
        response.error.unwrap().code,
        crate::protocol::error_code::IDENTITY_MISMATCH
    );
}

#[tokio::test]
async fn control_epoch_rejects_ambiguous_or_noncanonical_targets_before_auth() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(MALFORMED_CONTROL_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let session = terminal_id.to_string();
    let epoch = record.runtime_epoch.to_string();
    let session_field = yolo_capnp_value::Value::from(crate::protocol::field::TERMINAL_ID);
    let epoch_field = yolo_capnp_value::Value::from(crate::protocol::field::RUNTIME_EPOCH);
    let cases = [
        yolo_capnp_value::Value::Map(vec![
            (session_field.clone(), session.clone().into()),
            (session_field.clone(), session.clone().into()),
            (epoch_field.clone(), epoch.clone().into()),
        ]),
        yolo_capnp_value::Value::Map(vec![
            (session_field.clone(), session.clone().into()),
            (epoch_field.clone(), epoch.clone().into()),
            (epoch_field.clone(), epoch.clone().into()),
        ]),
        yolo_capnp_value::Value::Map(vec![
            (session_field.clone(), session.clone().into()),
            (epoch_field.clone(), yolo_capnp_value::Value::Null),
        ]),
        yolo_capnp_value::Value::Map(vec![
            (session_field.clone(), yolo_capnp_value::Value::Null),
            (epoch_field.clone(), epoch.clone().into()),
        ]),
        yolo_capnp_value::Value::Map(vec![
            (session_field.clone(), session.replace('-', "").into()),
            (epoch_field.clone(), epoch.clone().into()),
        ]),
        yolo_capnp_value::Value::Map(vec![
            (session_field.clone(), session.clone().into()),
            (epoch_field.clone(), epoch.replace('-', "").into()),
        ]),
        yolo_capnp_value::Value::Map(vec![(session_field.clone(), session.clone().into())]),
        yolo_capnp_value::Value::Map(vec![(epoch_field.clone(), epoch.clone().into())]),
        yolo_capnp_value::Value::Map(vec![
            (session_field.clone(), MALFORMED_UUID_TEXT.into()),
            (epoch_field.clone(), epoch.clone().into()),
        ]),
        yolo_capnp_value::Value::Map(vec![
            (session_field, session.into()),
            (epoch_field, MALFORMED_UUID_TEXT.into()),
        ]),
        yolo_capnp_value::Value::List(Vec::new()),
    ];
    for payload in cases {
        let request = RequestEnvelope::new(
            MALFORMED_CONTROL_REQUEST_ID,
            method::TERMINAL_INPUT,
            payload,
        );
        let response = host.dispatch(&request).await;
        assert!(!response.ok);
        assert_eq!(
            response.error.unwrap().code,
            crate::protocol::error_code::INVALID_REQUEST
        );
    }
    assert!(
        tokio::time::timeout(NO_WORKER_CONNECTION_TIMEOUT, listener.accept())
            .await
            .is_err()
    );
}
