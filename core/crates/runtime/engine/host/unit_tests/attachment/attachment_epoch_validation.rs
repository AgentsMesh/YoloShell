use crate::host::tests::*;
use super::*;

const MALFORMED_DIRECT_ATTACHMENT_EPOCH: &str = "not-a-canonical-runtime-epoch";
const POSITIONAL_ATTACHMENT_SEQUENCE: u64 = EMPTY_ATTACHMENT_NEXT_SEQUENCE;
const UNUSED_ATTACHMENT_SOCKET: &str = "unused-attachment.sock";
const OBSERVED_ATTACHMENT_SEQUENCE: u64 = 7;
const FORGED_ATTACHMENT_NEXT_SEQUENCE: u64 = 9;

fn attachment_epoch_field(
    value: yolo_capnp_value::Value,
) -> (yolo_capnp_value::Value, yolo_capnp_value::Value) {
    (
        yolo_capnp_value::Value::from(crate::protocol::field::RUNTIME_EPOCH),
        value,
    )
}

fn positional_attachment_payload(runtime_epoch: Uuid) -> yolo_capnp_value::Value {
    yolo_capnp_value::Value::List(vec![
        yolo_capnp_value::Value::from(runtime_epoch.to_string()),
        yolo_capnp_value::Value::Null,
        yolo_capnp_value::Value::List(Vec::new()),
        yolo_capnp_value::Value::from(POSITIONAL_ATTACHMENT_SEQUENCE),
        yolo_capnp_value::Value::Boolean(false),
    ])
}

#[test]
fn attachment_response_epoch_accepts_only_one_canonical_string_field() {
    let epoch = Uuid::new_v4();
    let canonical = yolo_capnp_value::Value::from(epoch.to_string());
    let duplicate = yolo_capnp_value::Value::Map(vec![
        attachment_epoch_field(canonical.clone()),
        attachment_epoch_field(canonical.clone()),
    ]);
    let cases = [
        (yolo_capnp_value::Value::Null, None),
        (yolo_capnp_value::Value::Map(Vec::new()), None),
        (
            yolo_capnp_value::Value::Map(vec![attachment_epoch_field(
                yolo_capnp_value::Value::Null,
            )]),
            None,
        ),
        (duplicate, None),
        (
            yolo_capnp_value::Value::Map(vec![attachment_epoch_field(
                yolo_capnp_value::Value::from(MALFORMED_DIRECT_ATTACHMENT_EPOCH),
            )]),
            None,
        ),
        (
            yolo_capnp_value::Value::Map(vec![attachment_epoch_field(canonical)]),
            Some(epoch),
        ),
    ];

    for (payload, expected) in cases {
        assert_eq!(attachment_response_epoch(&payload), expected);
    }
}

#[test]
fn worker_attachment_validation_checks_positional_runtime_identity() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let record = host_runtime_record(
        &host,
        terminal_id,
        directory.path().join(UNUSED_ATTACHMENT_SOCKET),
    );
    let request = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: u64::default(),
        },
    );
    let mut matching = ResponseEnvelope::empty(&request);
    matching.payload = positional_attachment_payload(record.runtime_epoch);
    assert!(validate_worker_attachment(&request, &matching, &record).is_ok());

    let mut mismatched = ResponseEnvelope::empty(&request);
    mismatched.payload = positional_attachment_payload(Uuid::new_v4());
    let error = validate_worker_attachment(&request, &mismatched, &record).unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
    assert!(!error.retryable);
}

#[test]
fn worker_attachment_validation_rejects_an_unobserved_cursor_advance() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let record = host_runtime_record(
        &host,
        terminal_id,
        directory.path().join(UNUSED_ATTACHMENT_SOCKET),
    );
    let request = request(
        method::ATTACH_SESSION,
        &AttachRequest {
            terminal_id,
            after_seq: OBSERVED_ATTACHMENT_SEQUENCE,
        },
    );
    let mut attached = empty_attachment(record.runtime_epoch);
    attached.next_sequence = FORGED_ATTACHMENT_NEXT_SEQUENCE;
    let response = ResponseEnvelope::success(&request, &attached).unwrap();

    let error = validate_worker_attachment(&request, &response, &record).unwrap_err();

    assert_eq!(
        error.code,
        crate::protocol::error_code::PROTOCOL_INCOMPATIBLE
    );
    assert!(!error.retryable);
}
