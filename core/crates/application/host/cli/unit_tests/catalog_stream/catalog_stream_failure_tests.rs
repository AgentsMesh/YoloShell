use super::*;
use crate::catalog_stream::catalog_stream_framing::{FrameError, read_catalog_frame};
use std::future::pending;
use std::time::Duration;
use yolo_runtime::cli::constants as cli;
use yolo_runtime::protocol::{ProtocolError, failure_kind, field, method};

const FRAME_ERROR_LENGTH: usize = 7;
const FRAME_IO_ERROR: &str = "injected catalog frame I/O failure";
const MALFORMED_FRAME_JSON: &str = "{";

#[test]
fn frame_errors_preserve_display_and_underlying_sources() {
    let io = FrameError::Io(std::io::Error::other(FRAME_IO_ERROR));
    let encode = FrameError::Encode(
        yolo_capnp_codec::decode::<serde_json::Value>(MALFORMED_FRAME_JSON.as_bytes()).unwrap_err(),
    );
    for error in [&io, &encode] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(error).is_some());
    }
    for error in [
        FrameError::EndOfStream,
        FrameError::TooLarge {
            length: FRAME_ERROR_LENGTH,
        },
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }
}

#[test]
fn remote_failure_preserves_typed_fields() {
    let code = method::LIST_SESSIONS.to_owned();
    let message = ProtocolError::ResponseMismatch.to_string();
    let failure = classify_failure(anyhow::Error::new(ClientError::Remote {
        code: code.clone(),
        message: message.clone(),
        retryable: true,
        details: None,
    }));
    let value = serde_json::to_value(failure).unwrap();
    assert_eq!(value[field::KIND], failure_kind::REMOTE);
    assert_eq!(value[field::CODE], code);
    assert_eq!(value[field::MESSAGE], message);
    assert_eq!(value[field::RETRYABLE], true);
    assert_eq!(value.as_object().unwrap().len(), 4);
}

#[tokio::test]
async fn protocol_and_timeout_failures_are_transport_typed() {
    let protocol = classify_failure(anyhow::Error::new(ClientError::Protocol(
        ProtocolError::ResponseMismatch,
    )));
    assert_transport_shape(protocol);
    assert_transport_shape(classify_failure(anyhow::Error::new(ClientError::Deadline)));

    let elapsed = tokio::time::timeout(Duration::ZERO, pending::<()>())
        .await
        .unwrap_err();
    assert_transport_shape(classify_failure(anyhow::Error::new(elapsed)));
}

#[test]
fn invalid_deadline_is_configuration_typed() {
    let error = catalog_deadline(u64::default()).unwrap_err();
    let value = serde_json::to_value(classify_failure(error)).unwrap();
    assert_eq!(value[field::KIND], failure_kind::CONFIGURATION);
    assert!(
        value[field::MESSAGE]
            .as_str()
            .is_some_and(|message| !message.is_empty())
    );
    assert_eq!(value.as_object().unwrap().len(), 2);
}

#[tokio::test]
async fn request_schema_rejects_unknown_duplicate_and_wrong_types() {
    use yolo_capnp_codec::Value;
    let valid_id = "00000000-0000-0000-0000-000000000001";
    let base = vec![
        (Value::from(field::ID), Value::from(valid_id)),
        (Value::from(field::COMMAND), Value::from(cli::LIST_COMMAND)),
        (Value::from(field::DEADLINE_MS), Value::from(1u64)),
    ];
    let mut unknown = base.clone();
    unknown.push((Value::from(method::HOST_STATUS), Value::Boolean(true)));
    let mut duplicate = base.clone();
    duplicate.push((Value::from(field::DEADLINE_MS), Value::from(2u64)));
    let mut wrong_type = base;
    wrong_type.last_mut().unwrap().1 = Value::from("1");
    for fields in [unknown, duplicate, wrong_type] {
        let payload = yolo_capnp_codec::encode_value(&Value::Map(fields)).unwrap();
        let error = decode_request(&payload).await.unwrap_err();
        assert!(matches!(error, FrameError::Encode(_)));
    }
}

fn assert_transport_shape(failure: CatalogFailure) {
    let value = serde_json::to_value(failure).unwrap();
    assert_eq!(value[field::KIND], failure_kind::TRANSPORT);
    assert!(
        value[field::MESSAGE]
            .as_str()
            .is_some_and(|message| !message.is_empty())
    );
    assert_eq!(value.as_object().unwrap().len(), 2);
}

async fn decode_request(payload: &[u8]) -> Result<CatalogRequest, FrameError> {
    let mut frame = Vec::from((payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    read_catalog_frame(&mut frame.as_slice()).await
}
