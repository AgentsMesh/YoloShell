use crate::json::{decode, encode};
use serde_json::{Value, json};

#[test]
fn encodes_stable_pretty_json_with_a_trailing_newline() {
    let value = json!({"product": "Yolo", "version": 1});
    let bytes = encode(&value).unwrap();
    assert!(bytes.ends_with(b"\n"));
    assert!(
        String::from_utf8(bytes)
            .unwrap()
            .contains("  \"product\": \"Yolo\"")
    );
}

#[test]
fn decodes_valid_json_and_rejects_invalid_input() {
    let value: Value = decode(br#"{"product":"Yolo"}"#).unwrap();
    assert_eq!(value["product"], "Yolo");
    assert!(decode::<Value>(b"not-json").is_err());
}
