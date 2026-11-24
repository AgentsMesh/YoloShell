use super::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
struct NestedValue(Vec<NestedValue>);

#[test]
fn strict_decoder_rejects_excessive_capnp_nesting() {
    let mut value = NestedValue(Vec::new());
    for _ in 0..=MAX_PROCESS_SNAPSHOT_NESTING_DEPTH {
        value = NestedValue(vec![value]);
    }
    let encoded = crate::protocol::encode(&value).unwrap();
    let payload = encoded[crate::protocol::FRAME_LENGTH_BYTES..].to_vec();

    assert!(decode_strict::<NestedValue>(&payload).is_err());
}
