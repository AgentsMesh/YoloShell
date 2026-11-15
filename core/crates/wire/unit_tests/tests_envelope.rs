use super::envelope::REDACTED_ENVELOPE_PAYLOAD;
use super::{
    ErrorBody, EventEnvelope, FRAME_LENGTH_BYTES, PROTOCOL_VERSION, ProtocolError, RequestEnvelope,
    ResponseEnvelope, decode, encode, from_value, to_value,
};
use serde::{Deserialize, Serialize};
use yolo_capnp_value::Value;

fn roundtrip<T>(value: &T) -> T
where
    T: Serialize + for<'de> Deserialize<'de>,
{
    decode(&encode(value).unwrap()[FRAME_LENGTH_BYTES..]).unwrap()
}

#[test]
fn all_envelope_shapes_roundtrip() {
    let request = RequestEnvelope::new("request", super::method::HELLO, Value::Null);
    assert_eq!(roundtrip(&request), request);
    let missing_payload = Value::Map(vec![
        (
            Value::from(super::field::VERSION),
            Value::from(PROTOCOL_VERSION),
        ),
        (
            Value::from(super::field::KIND),
            Value::from(super::REQUEST_KIND),
        ),
        (
            Value::from(super::field::REQUEST_ID),
            Value::from("request"),
        ),
        (
            Value::from(super::field::METHOD),
            Value::from(super::method::HELLO),
        ),
    ]);
    assert!(from_value::<RequestEnvelope>(missing_payload).is_err());

    let success = ResponseEnvelope::success(&request, &7_u8).unwrap();
    assert_eq!(roundtrip(&success).decode_payload::<u8>().unwrap(), 7);
    assert_eq!(
        roundtrip(&ResponseEnvelope::empty(&request)).payload,
        Value::Null
    );
    let failure = ResponseEnvelope::failure(
        &request,
        ErrorBody {
            code: super::error_code::CONFLICT.into(),
            message: "conflict".into(),
            retryable: false,
            details: Some(serde_json::json!({"holder": "desktop"})),
        },
    );
    assert_eq!(
        roundtrip(&failure).error.unwrap().details,
        Some(serde_json::json!({"holder": "desktop"}))
    );

    let event = EventEnvelope {
        version: PROTOCOL_VERSION,
        kind: super::EVENT_KIND.into(),
        event: "output".into(),
        terminal_id: Some("session".into()),
        runtime_epoch: Some("runtime".into()),
        payload: Value::from(1),
    };
    assert_eq!(roundtrip(&event).terminal_id.as_deref(), Some("session"));
    let event = EventEnvelope::new("output", None, None, &()).unwrap();
    assert_eq!(roundtrip(&event).runtime_epoch, None);
}

#[test]
fn response_envelope_rejects_unknown_and_duplicate_fields() {
    let request = RequestEnvelope::new("strict-response", "strict-method", Value::Null);
    let response = ResponseEnvelope::empty(&request);
    let Value::Map(fields) = to_value(&response).unwrap() else {
        panic!("response must encode as a map");
    };
    let mut unknown = fields.clone();
    unknown.push((Value::from("unknown"), Value::Null));
    assert!(from_value::<ResponseEnvelope>(Value::Map(unknown)).is_err());
    let mut duplicate = fields;
    duplicate.push((
        Value::from(super::field::KIND),
        Value::from(super::RESPONSE_KIND),
    ));
    assert!(from_value::<ResponseEnvelope>(Value::Map(duplicate)).is_err());
}

#[test]
fn every_error_body_constructor_roundtrips() {
    for body in [
        ErrorBody::invalid("invalid"),
        ErrorBody::not_found("missing"),
        ErrorBody::conflict("conflict"),
        ErrorBody::internal("internal"),
        ErrorBody::new("unavailable", "offline", true),
        ErrorBody::lease(
            crate::error_code::LEASE_CONFLICT,
            "controlled",
            serde_json::json!({"generation": 4}),
        ),
    ] {
        let decoded: ErrorBody = from_value(to_value(&body).unwrap()).unwrap();
        assert_eq!(decoded, body);
    }
}

#[test]
fn protocol_error_variants_have_stable_diagnostics() {
    let encode_error = to_value(&FailingSerialize).unwrap_err();
    assert!(encode_error.to_string().contains("protocol encode"));
    let decode_error = decode::<Value>(&[0xc1]).unwrap_err();
    assert!(decode_error.to_string().contains("protocol decode"));
    assert!(
        ProtocolError::FrameTooLarge { length: 17 }
            .to_string()
            .contains("17")
    );
    assert!(
        ProtocolError::Io(std::io::Error::other("closed"))
            .to_string()
            .contains("closed")
    );
    assert_eq!(
        ProtocolError::EndOfStream.to_string(),
        "protocol stream ended"
    );
    assert!(
        ProtocolError::Remote(ErrorBody::not_found("missing"))
            .to_string()
            .contains(crate::error_code::NOT_FOUND)
    );
}

#[test]
fn envelope_debug_never_exposes_payload_values() {
    const SECRET: &str = "d9428888-122b-4b8a-b56f-97031ebce3bf";
    let request = RequestEnvelope::new("debug-request", "secret-method", SECRET.into());
    let response = ResponseEnvelope::success(&request, &SECRET).unwrap();
    let event = EventEnvelope::new("secret-event", None, None, &SECRET).unwrap();

    for debug in [
        format!("{request:?}"),
        format!("{response:?}"),
        format!("{event:?}"),
    ] {
        assert!(debug.contains(REDACTED_ENVELOPE_PAYLOAD));
        assert!(!debug.contains(SECRET));
    }
}

#[derive(Debug)]
struct FailingSerialize;

impl Serialize for FailingSerialize {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        Err(serde::ser::Error::custom("intentional failure"))
    }
}
