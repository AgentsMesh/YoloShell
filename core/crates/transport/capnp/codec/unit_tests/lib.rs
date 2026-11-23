use serde::{Deserialize, Serialize};
use yolo_capnp_codec::{Value, decode, decode_value, encode, encode_value};
mod adversarial;
mod budgets;
mod nesting;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Document {
    number: u64,
    name: String,
    #[serde(with = "serde_bytes")]
    output: Vec<u8>,
}

#[test]
fn schema_document_preserves_utf8_bytes_and_wide_values() {
    let document = Document {
        number: u64::MAX,
        name: "终端".into(),
        output: vec![0, 255, 27],
    };
    let bytes = encode(&document).unwrap();
    assert_eq!(decode::<Document>(&bytes).unwrap(), document);
    assert!(matches!(decode_value(&bytes).unwrap(), Value::Map(_)));
}

#[test]
fn trailing_truncated_and_wrong_schema_are_rejected() {
    let bytes = encode(&true).unwrap();
    for length in 0..bytes.len() {
        assert!(decode::<bool>(&bytes[..length]).is_err());
    }
    let mut extra = bytes.clone();
    extra.extend_from_slice(&bytes);
    assert!(decode::<bool>(&extra).is_err());
    let mut builder = capnp::message::Builder::new_default();
    builder
        .init_root::<yolo_capnp_schema::value_capnp::document::Builder<'_>>()
        .set_version(u16::MAX);
    assert!(decode::<bool>(&capnp::serialize::write_message_to_words(&builder)).is_err());
}

#[test]
fn deeply_nested_and_duplicate_maps_cannot_bypass_domain_validation() {
    let mut value = Value::Null;
    for _ in 0..=yolo_capnp_codec::MAX_VALUE_DEPTH {
        value = Value::List(vec![value]);
    }
    assert!(encode_value(&value).is_err());
    let duplicate = Value::Map(vec![
        (Value::from("id"), Value::Unsigned(1)),
        (Value::from("id"), Value::Unsigned(2)),
    ]);
    let bytes = encode_value(&duplicate).unwrap();
    assert!(decode::<std::collections::BTreeMap<String, u64>>(&bytes).is_err());
}

#[test]
fn scalar_variants_roundtrip_without_text_wrapping() {
    for value in [
        Value::Null,
        Value::Boolean(true),
        Value::Signed(i64::MIN),
        Value::Unsigned(u64::MAX),
        Value::Float(1.5),
        Value::Text("text".into()),
        Value::Data(vec![255]),
        Value::List(vec![]),
        Value::Map(vec![]),
    ] {
        assert_eq!(decode_value(&encode_value(&value).unwrap()).unwrap(), value);
    }
}
