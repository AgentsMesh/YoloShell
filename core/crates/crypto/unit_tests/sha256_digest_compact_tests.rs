use crate::constants::{NONCANONICAL_PADDING, TEST_DIGEST_BYTE};
use serde::{Deserialize, Serialize};
use yolo_crypto::Sha256Digest;

#[derive(Deserialize, Serialize)]
struct Wire(#[serde(with = "yolo_crypto::sha256_digest::compact")] Sha256Digest);

#[test]
fn digest_round_trips_as_canonical_unpadded_base64url() {
    let digest = Sha256Digest::repeated(TEST_DIGEST_BYTE);
    let encoded = serde_json::to_value(Wire(digest)).unwrap();
    assert!(
        encoded
            .as_str()
            .is_some_and(|value| !value.contains(NONCANONICAL_PADDING))
    );
    assert_eq!(serde_json::from_value::<Wire>(encoded).unwrap().0, digest);
}

#[test]
fn padded_and_truncated_encodings_are_rejected() {
    let digest = Sha256Digest::repeated(TEST_DIGEST_BYTE);
    let encoded = serde_json::to_value(Wire(digest)).unwrap();
    let mut padded = encoded.as_str().unwrap().to_owned();
    padded.push(NONCANONICAL_PADDING);
    assert!(serde_json::from_value::<Wire>(serde_json::Value::String(padded)).is_err());

    let mut truncated = encoded.as_str().unwrap().to_owned();
    truncated.pop();
    assert!(serde_json::from_value::<Wire>(serde_json::Value::String(truncated)).is_err());
}
