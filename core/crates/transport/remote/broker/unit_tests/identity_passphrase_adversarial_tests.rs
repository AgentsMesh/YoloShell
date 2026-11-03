use super::*;
use serde_json::{Value, json};

const HOST_ID: &str = "binding-host";
const NONCE: &str = "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa";
const REVISION: &str = "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb";
const DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const INVALID_NONCE: &str = "aaaaaaaa-aaaa-1aaa-8aaa-aaaaaaaaaaaa";
const INVALID_HOST: &str = "../host";
const INVALID_REVISION: &str = "invalid";
const INVALID_DIGEST: &str = "ABC";
const EXTRA_FIELD: &str = "extra";

fn envelope() -> Value {
    json!({
        "version": WIRE_VERSION,
        (BINDING_NONCE_FIELD): NONCE,
        (HOST_ID_FIELD): HOST_ID,
        (CREDENTIAL_REVISION_FIELD): REVISION,
        (IDENTITY_PATHS_DIGEST_FIELD): DIGEST,
        "passphrases": [],
    })
}

#[test]
fn malformed_binding_fields_and_unknown_shape_fail_closed() {
    let mutations = [
        (BINDING_NONCE_FIELD, json!(INVALID_NONCE)),
        (HOST_ID_FIELD, json!(INVALID_HOST)),
        (CREDENTIAL_REVISION_FIELD, json!(INVALID_REVISION)),
        (IDENTITY_PATHS_DIGEST_FIELD, json!(INVALID_DIGEST)),
        (EXTRA_FIELD, json!(true)),
    ];
    for (field, value) in mutations {
        let mut envelope = envelope();
        envelope[field] = value;
        let encoded = yolo_capnp_codec::encode(&envelope).unwrap();
        assert!(IdentityPassphrases::read_checked(encoded.as_slice(), false).is_err());
    }
}

#[test]
fn identity_count_limit_is_enforced_before_secret_use() {
    let mut envelope = envelope();
    envelope["passphrases"] = json!(vec![None::<String>; MAX_IDENTITY_FILES + 1]);
    let encoded = yolo_capnp_codec::encode(&envelope).unwrap();
    assert!(IdentityPassphrases::read_checked(encoded.as_slice(), false).is_err());
}

#[test]
fn wire_rejects_duplicate_keys_plain_json_and_secret_bearing_wrong_types_without_disclosure() {
    use yolo_capnp_codec::Value as WireValue;
    let source = envelope();
    let WireValue::Map(fields) = yolo_capnp_codec::to_value(&source).unwrap() else {
        panic!()
    };
    let mut duplicate = fields.clone();
    duplicate.push(fields[0].clone());
    let bytes = yolo_capnp_codec::encode_value(&WireValue::Map(duplicate)).unwrap();
    assert!(IdentityPassphrases::read_checked(bytes.as_slice(), false).is_err());
    assert!(
        IdentityPassphrases::read_checked(serde_json::to_vec(&source).unwrap().as_slice(), false)
            .is_err()
    );
    const SECRET: &str = "never-display-this-private-value";
    for field in [VERSION_FIELD, PASSPHRASES_FIELD] {
        let mut wrong = source.clone();
        wrong[field] = json!(SECRET);
        let bytes = yolo_capnp_codec::encode(&wrong).unwrap();
        let error = IdentityPassphrases::read_checked(bytes.as_slice(), false)
            .err()
            .unwrap();
        assert!(!format!("{error:#}").contains(SECRET));
    }
}
