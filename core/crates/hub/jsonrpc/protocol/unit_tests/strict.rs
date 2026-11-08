use serde_json::{Value, json};
use yolo_hub_jsonrpc_protocol::decode;

#[test]
fn recursive_duplicate_keys_trailing_documents_and_excessive_depth_are_rejected() {
    for bytes in [
        b"{\"id\":1,\"id\":2}".as_slice(),
        b"{\"params\":{\"request_id\":1,\"request_id\":2}}",
        b"[{\"x\":1,\"x\":2}]",
        b"{}{}",
        b"NaN",
        b"",
        b"\xff",
    ] {
        assert!(decode::<Value>(bytes).is_err());
    }
    const EXCESSIVE_DEPTH: usize = 256;
    assert!(
        decode::<Value>(
            format!(
                "{}{}",
                "[".repeat(EXCESSIVE_DEPTH),
                "]".repeat(EXCESSIVE_DEPTH)
            )
            .as_bytes()
        )
        .is_err()
    );
}

#[test]
fn all_json_scalar_types_and_nested_collections_preserve_their_values() {
    let value = json!({"string":"escaped\ntext", "boolean":true, "nothing":null,
        "negative":-2, "positive":u64::MAX, "fraction":1.25, "nested":[{},[],false]});
    let bytes = serde_json::to_vec(&value).unwrap();
    assert_eq!(decode::<Value>(&bytes).unwrap(), value);
}
