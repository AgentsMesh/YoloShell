use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;
use uuid::Uuid;
use yolo_capnp_value::Value;
use yolo_runtime::protocol::{
    FRAME_LENGTH_BYTES, MAX_FRAME_BYTES, PROTOCOL_VERSION, ProtocolError, RequestEnvelope, decode,
    encode, field, from_value, read_frame_blocking, to_value,
};

const EXTRA_PAYLOAD_VALUE: u8 = 99;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Fixture {
    terminal_id: Uuid,
    name: String,
    #[serde(with = "serde_bytes")]
    bytes: Vec<u8>,
}

#[test]
fn internal_payloads_are_named_maps_with_string_ids_and_binary_terminal_data() {
    let id = Uuid::new_v4();
    let fixture = Fixture {
        terminal_id: id,
        name: "terminal".into(),
        bytes: vec![0, 0xff, 0x1b],
    };
    let value = to_value(&fixture).unwrap();
    let map = value.as_map().expect("struct payload must be a map");
    assert_eq!(
        map.iter()
            .find(|(key, _)| key.as_str() == Some(field::TERMINAL_ID))
            .unwrap()
            .1
            .as_str(),
        Some(id.to_string().as_str())
    );
    assert_eq!(
        map.iter()
            .find(|(key, _)| key.as_str() == Some("bytes"))
            .unwrap()
            .1
            .as_slice(),
        Some([0, 0xff, 0x1b].as_slice())
    );
    assert_eq!(from_value::<Fixture>(value).unwrap(), fixture);
}

#[test]
fn internal_frame_round_trip_preserves_structural_payload_fields() {
    let payload = Value::Map(vec![
        (Value::from(field::OK), Value::from(true)),
        (
            Value::from(field::NAME),
            Value::from(EXTRA_PAYLOAD_VALUE),
        ),
    ]);
    let request = RequestEnvelope::new("request-1", "hello", payload.clone());
    let framed = encode(&request).unwrap();
    let decoded: RequestEnvelope = decode(&framed[FRAME_LENGTH_BYTES..]).unwrap();
    let decoded_payload: std::collections::BTreeMap<String, Value> =
        from_value(decoded.payload).unwrap();
    assert_eq!(decoded_payload, from_value(payload).unwrap());
    assert_eq!(decoded.version, PROTOCOL_VERSION);
    assert_eq!(decoded.kind, "request");
}

#[test]
fn binary_is_not_base64_or_integer_array() {
    let value = to_value(&Fixture {
        terminal_id: Uuid::nil(),
        name: String::new(),
        bytes: vec![1, 2, 3],
    })
    .unwrap();
    let encoded = encode(&value).unwrap();
    let decoded: std::collections::BTreeMap<String, yolo_capnp_value::Value> =
        decode(&encoded[FRAME_LENGTH_BYTES..]).unwrap();
    assert!(matches!(decoded["bytes"], Value::Data(_)));
    let binary: ByteBuf = from_value(decoded["bytes"].clone()).unwrap();
    assert_eq!(binary.as_ref(), &[1, 2, 3]);
}

#[test]
fn oversized_declared_frame_is_rejected_without_allocating_payload() {
    let input = ((MAX_FRAME_BYTES + 1) as u32).to_be_bytes();
    let error = read_frame_blocking::<_, RequestEnvelope>(&mut input.as_slice()).unwrap_err();
    assert!(matches!(
        error,
        ProtocolError::FrameTooLarge { length } if length == MAX_FRAME_BYTES + 1
    ));
}
