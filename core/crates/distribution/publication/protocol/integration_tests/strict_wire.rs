use super::{constants::*, fixtures::*};
use serde_json::{Value, json};
use std::path::Path;
use uuid::Uuid;
use yolo_distribution_publication_protocol::*;

#[test]
fn bounded_strict_wire_roundtrips_and_rejects_missing_unknown_or_duplicate_keys() {
    let request = request();
    let receipt =
        PublicationReceipt::new(&request, Path::new(ROOT), request.selected().unwrap()).unwrap();
    let inspection = InspectionReceipt::new(
        Path::new(ROOT),
        Some((request.selected().unwrap(), request.descriptor().clone())),
    )
    .unwrap();
    assert_eq!(
        decode_request(&encode_request(&request).unwrap()).unwrap(),
        request
    );
    assert_eq!(decode_request_argument(&encode_request_argument(&request).unwrap()).unwrap(), request);
    assert!(decode_request(&serde_json::to_vec(&request).unwrap()).is_err());
    assert!(decode_request_argument(&serde_json::to_string(&request).unwrap()).is_err());
    assert_eq!(
        decode_receipt(&encode_receipt(&receipt).unwrap()).unwrap(),
        receipt
    );
    assert_eq!(
        decode_inspection(&encode_inspection(&inspection).unwrap()).unwrap(),
        inspection
    );
    for malformed in malformed_capnp(&request) {
        assert!(decode_request(&malformed).is_err());
    }
    for malformed in malformed_capnp(&receipt) {
        assert!(decode_receipt(&malformed).is_err());
    }
    for malformed in malformed_capnp(&inspection) {
        assert!(decode_inspection(&malformed).is_err());
    }
    for malformed in malformed_objects(&serde_json::to_value(request.selected().unwrap()).unwrap())
    {
        assert!(decode_selection(&malformed).is_err());
    }
    let overlong = vec![b' '; MAX_PUBLICATION_BYTES + 1];
    assert!(decode_request(&overlong).is_err());
    assert!(decode_receipt(&overlong).is_err());
    assert!(decode_inspection(&overlong).is_err());
    assert!(decode_selection(&overlong).is_err());
}

#[test]
fn nil_schema_and_nested_selection_tampering_are_not_deserialized_as_valid_requests() {
    for (key, value) in [
        (SCHEMA_KEY, json!(PUBLICATION_SCHEMA_VERSION + 1)),
        (OPERATION_KEY, json!(Uuid::nil())),
        (STAGE_KEY, json!(Uuid::nil())),
    ] {
        let mut value_json = serde_json::to_value(request()).unwrap();
        value_json[key] = value;
        assert!(decode_request(&yolo_capnp_codec::encode(&value_json).unwrap()).is_err());
    }
    let source = request();
    let mut fields = source.selected().unwrap().fields().clone();
    fields.build_id = INVALID_DIGEST.into();
    assert!(Selection::try_from(fields).is_err());
    let mut fields = source.selected().unwrap().fields().clone();
    fields.revision = 0;
    assert!(Selection::try_from(fields).is_err());
    let mut fields = source.selected().unwrap().fields().clone();
    fields.schema_version += 1;
    assert!(Selection::try_from(fields).is_err());
    let mut fields = source.selected().unwrap().fields().clone();
    fields.operation_id = Uuid::nil();
    assert!(Selection::try_from(fields).is_err());
}

fn malformed_capnp(value: &impl serde::Serialize) -> Vec<Vec<u8>> {
    use yolo_capnp_codec::Value;
    let Value::Map(fields) = yolo_capnp_codec::to_value(value).unwrap() else {
        panic!()
    };
    let mut malformed = Vec::new();
    for index in 0..fields.len() {
        let mut missing = fields.clone();
        missing.remove(index);
        malformed.push(yolo_capnp_codec::encode_value(&Value::Map(missing)).unwrap());
        let mut duplicate = fields.clone();
        duplicate.push(fields[index].clone());
        malformed.push(yolo_capnp_codec::encode_value(&Value::Map(duplicate)).unwrap());
    }
    let mut unknown = fields;
    unknown.push((Value::from(EXTRA_KEY), Value::from(EXTRA_VALUE)));
    malformed.push(yolo_capnp_codec::encode_value(&Value::Map(unknown)).unwrap());
    malformed
}

fn malformed_objects(value: &Value) -> Vec<Vec<u8>> {
    let encoded = serde_json::to_string(value).unwrap();
    let mut malformed = Vec::new();
    for (key, field_value) in value.as_object().unwrap() {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(key);
        malformed.push(serde_json::to_vec(&missing).unwrap());
        let field = format!("{}:{}", serde_json::to_string(key).unwrap(), field_value);
        malformed.push(
            encoded
                .replacen(&field, &format!("{field},{field}"), 1)
                .into_bytes(),
        );
    }
    let mut unknown = value.clone();
    unknown[EXTRA_KEY] = json!(EXTRA_VALUE);
    malformed.push(serde_json::to_vec(&unknown).unwrap());
    malformed
}
