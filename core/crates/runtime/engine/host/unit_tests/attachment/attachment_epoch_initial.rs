use crate::host::tests::*;
use super::*;

const MALFORMED_ATTACHMENT_EPOCH: &str = "malformed-attachment-runtime-epoch";
const ATTACHMENT_SOCKET_NAME: &str = "attachment-initial.sock";
const INITIAL_ATTACH_AFTER_SEQUENCE: u64 = 0;

#[derive(Clone, Copy)]
enum InitialEpochCorruption {
    Missing,
    Malformed,
    Wrong,
    Nil,
}

#[tokio::test]
async fn attachment_epoch_host_rejects_invalid_initial_before_forwarding() {
    for corruption in [
        InitialEpochCorruption::Missing,
        InitialEpochCorruption::Malformed,
        InitialEpochCorruption::Wrong,
        InitialEpochCorruption::Nil,
    ] {
        assert_initial_epoch_rejected(corruption).await;
    }
}

async fn assert_initial_epoch_rejected(corruption: InitialEpochCorruption) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let socket = directory.path().join(ATTACHMENT_SOCKET_NAME);
    let listener = UnixListener::bind(&socket).unwrap();
    let (terminal_id, record) = attachment_record(&host, socket);
    let payload = corrupted_initial_payload(record.runtime_epoch, corruption);
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut stream = UnixStream::from(stream);
        let mut stream = accept_worker_auth(stream, &record).await;
        let request: RequestEnvelope = stream.request().await.unwrap();
        let mut response = ResponseEnvelope::empty(&request);
        response.payload = payload;
        stream.respond(&response).await.unwrap();
    });
    let error = host
        .rpc_attachment(&request(
            method::ATTACH_SESSION,
            &AttachRequest {
                terminal_id,
                after_seq: INITIAL_ATTACH_AFTER_SEQUENCE,
            },
        ))
        .await
        .err()
        .unwrap();
    assert!(!error.retryable);
    assert_eq!(error.code, expected_initial_epoch_error(corruption));
    server.await.unwrap();
}

fn corrupted_initial_payload(
    expected: Uuid,
    corruption: InitialEpochCorruption,
) -> yolo_capnp_value::Value {
    let actual = match corruption {
        InitialEpochCorruption::Wrong => Uuid::new_v4(),
        InitialEpochCorruption::Nil => Uuid::nil(),
        InitialEpochCorruption::Missing | InitialEpochCorruption::Malformed => expected,
    };
    let yolo_capnp_value::Value::Map(mut fields) = to_value(&empty_attachment(actual)).unwrap()
    else {
        panic!("attach result must encode as a map");
    };
    if matches!(
        corruption,
        InitialEpochCorruption::Missing | InitialEpochCorruption::Malformed
    ) {
        fields.retain(|(key, _)| key.as_str() != Some(crate::protocol::field::RUNTIME_EPOCH));
    }
    if matches!(corruption, InitialEpochCorruption::Malformed) {
        fields.push((
            yolo_capnp_value::Value::from(crate::protocol::field::RUNTIME_EPOCH),
            yolo_capnp_value::Value::from(MALFORMED_ATTACHMENT_EPOCH),
        ));
    }
    yolo_capnp_value::Value::Map(fields)
}

fn expected_initial_epoch_error(corruption: InitialEpochCorruption) -> &'static str {
    match corruption {
        InitialEpochCorruption::Missing | InitialEpochCorruption::Malformed => {
            crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
        }
        InitialEpochCorruption::Wrong | InitialEpochCorruption::Nil => {
            crate::protocol::error_code::IDENTITY_MISMATCH
        }
    }
}
