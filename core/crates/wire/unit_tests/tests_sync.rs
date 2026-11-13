use super::*;
use serde::{Deserialize, Serialize};
use yolo_capnp_value::Value;

#[test]
fn frame_is_big_endian_length_prefixed_named_map() {
    let request = RequestEnvelope::new("id", method::HELLO, Value::Null);
    let bytes = encode(&request).unwrap();
    assert_eq!(
        u32::from_be_bytes(bytes[..FRAME_LENGTH_BYTES].try_into().unwrap()) as usize,
        bytes.len() - FRAME_LENGTH_BYTES
    );
    assert_eq!(
        decode::<RequestEnvelope>(&bytes[FRAME_LENGTH_BYTES..]).unwrap(),
        request
    );
}

#[test]
fn payload_preserves_capnp_data() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct BinaryPayload {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }
    let payload = BinaryPayload {
        data: vec![0, 159, 255],
    };
    let value = to_value(&payload).unwrap();
    assert_eq!(from_value::<BinaryPayload>(value.clone()).unwrap(), payload);
    let Value::Map(fields) = value else {
        panic!("binary payload must remain a named map");
    };
    let data = fields
        .into_iter()
        .find(|(key, _)| key.as_str() == Some("data"))
        .map(|(_, value)| value)
        .unwrap();
    assert_eq!(data.as_slice(), Some([0, 159, 255].as_slice()));
}

#[test]
fn envelope_constructors_are_wire_stable() {
    let request = RequestEnvelope::new("id", method::HELLO, Value::Null);
    let success = ResponseEnvelope::success(&request, &serde_json::json!({"ok": true})).unwrap();
    assert!(success.ok);
    let empty = ResponseEnvelope::empty(&request);
    assert_eq!(empty.decode_payload::<()>().unwrap(), ());
    for error in [
        ErrorBody::invalid("invalid"),
        ErrorBody::not_found("missing"),
        ErrorBody::conflict("held"),
        ErrorBody::internal("broken"),
    ] {
        let failure = ResponseEnvelope::failure(&request, error.clone());
        assert!(!failure.ok);
        assert_eq!(failure.error, Some(error));
    }
    let lease = ErrorBody::lease(
        crate::error_code::LEASE_CONFLICT,
        "held",
        serde_json::json!({"generation": 2}),
    );
    assert_eq!(lease.details.unwrap()["generation"], 2);
    let event = EventEnvelope::new(
        event::SESSION_UPDATED,
        Some("session".into()),
        Some("epoch".into()),
        &serde_json::json!({"state": "running"}),
    )
    .unwrap();
    assert_eq!(event.kind, "event");
}

#[test]
fn blocking_round_trip_and_decode_errors_are_explicit() {
    let request = RequestEnvelope::new("blocking", "hello", Value::Null);
    let mut bytes = Vec::new();
    write_frame_blocking(&mut bytes, &request).unwrap();
    assert_eq!(
        read_frame_blocking::<_, RequestEnvelope>(&mut bytes.as_slice()).unwrap(),
        request
    );
    assert!(matches!(
        read_frame_blocking::<_, RequestEnvelope>(&mut [].as_slice()),
        Err(ProtocolError::EndOfStream)
    ));
    assert!(decode::<RequestEnvelope>(&[0xc1]).is_err());
    assert!(from_value::<RequestEnvelope>(Value::Boolean(true)).is_err());
}

#[test]
fn frame_limits_are_checked_before_allocation() {
    let mut bytes = ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes().to_vec();
    bytes.push(0);
    assert!(matches!(
        read_frame_blocking::<_, RequestEnvelope>(&mut bytes.as_slice()),
        Err(ProtocolError::FrameTooLarge { .. })
    ));
    #[derive(Serialize)]
    struct Large {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }
    assert!(matches!(
        encode(&Large {
            data: vec![0; MAX_FRAME_BYTES + 1]
        }),
        Err(ProtocolError::FrameTooLarge { .. })
    ));
    assert!(matches!(
        decode::<Value>(&vec![0; MAX_FRAME_BYTES + 1]),
        Err(ProtocolError::FrameTooLarge { .. })
    ));
}

#[test]
fn truncated_payload_and_failing_io_surface_io_errors() {
    let truncated = vec![0, 0, 0, 5, 0x80];
    assert!(matches!(
        read_frame_blocking::<_, RequestEnvelope>(&mut truncated.as_slice()),
        Err(ProtocolError::Io(_))
    ));

    struct FailingIo;
    impl std::io::Read for FailingIo {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::PermissionDenied.into())
        }
    }
    impl std::io::Write for FailingIo {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    assert!(matches!(
        read_frame_blocking::<_, RequestEnvelope>(&mut FailingIo),
        Err(ProtocolError::Io(_))
    ));
    assert!(matches!(
        write_frame_blocking(
            &mut FailingIo,
            &RequestEnvelope::new("id", method::HELLO, Value::Null)
        ),
        Err(ProtocolError::Io(_))
    ));
}

#[test]
fn missing_payload_is_rejected_instead_of_inventing_nil() {
    let value = Value::Map(vec![
        (
            Value::from(super::field::VERSION),
            Value::from(PROTOCOL_VERSION),
        ),
        (
            Value::from(super::field::KIND),
            Value::from(super::REQUEST_KIND),
        ),
        (Value::from(super::field::REQUEST_ID), Value::from("id")),
        (
            Value::from(super::field::METHOD),
            Value::from(method::HELLO),
        ),
    ]);
    assert!(from_value::<RequestEnvelope>(value).is_err());
}
