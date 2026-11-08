use crate::constants::{FIRST_PAYLOAD_VALUE, SECOND_PAYLOAD_VALUE, TEST_DOMAIN};
use yolo_crypto::Sha256Digest;

#[test]
fn digest_is_stable_and_payload_sensitive() {
    let first = Sha256Digest::capture(&(TEST_DOMAIN, FIRST_PAYLOAD_VALUE)).unwrap();
    let replay = Sha256Digest::capture(&(TEST_DOMAIN, FIRST_PAYLOAD_VALUE)).unwrap();
    let changed = Sha256Digest::capture(&(TEST_DOMAIN, SECOND_PAYLOAD_VALUE)).unwrap();
    assert_eq!(first, replay);
    assert_ne!(first, changed);
}

#[test]
fn digest_round_trips_through_serde() {
    let digest = Sha256Digest::capture(&TEST_DOMAIN).unwrap();
    let encoded = serde_json::to_vec(&digest).unwrap();
    assert_eq!(
        serde_json::from_slice::<Sha256Digest>(&encoded).unwrap(),
        digest
    );
}
