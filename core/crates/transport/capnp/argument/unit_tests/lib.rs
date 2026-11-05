use yolo_capnp_argument::{decode, encode};

const LIMIT: usize = 1024;

#[test]
fn bounded_argument_roundtrip_rejects_plain_json_non_base64_and_size_overflow() {
    let value = vec![u64::MAX, 0, 1];
    let encoded = encode(&value, LIMIT).unwrap();
    assert_eq!(decode::<Vec<u64>>(&encoded, LIMIT).unwrap(), value);
    assert!(decode::<Vec<u64>>("[]", LIMIT).is_err());
    assert!(decode::<Vec<u64>>(&encoded, 1).is_err());
    assert!(encode(&value, 1).is_err());
    assert!(decode::<Vec<u64>>(&"a".repeat(LIMIT * 2), LIMIT).is_err());
}
