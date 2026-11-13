use yolo_capnp_codec::{
    Value, ValueError, canonical, decode, decode_value, decode_with_depth, encode, encode_value,
};

const WORD_BYTES: usize = std::mem::size_of::<u64>();
const HEADER_BYTES: usize = 2 * std::mem::size_of::<u32>();
const OVERSIZED_WORDS: u32 = (yolo_capnp_codec::MAX_MESSAGE_BYTES / WORD_BYTES + 1) as u32;
const UNALIGNED_OFFSET: usize = 3;
const TEXT_SENTINEL: &str = "canonical-utf8-sentinel";
const INVALID_UTF8: u8 = 0xff;
const CHILD_DEPTH: usize = 1;

#[test]
fn hostile_segment_lengths_are_rejected_without_reading_or_allocating_the_body() {
    let mut header = [u8::default(); HEADER_BYTES];
    header[std::mem::size_of::<u32>()..].copy_from_slice(&OVERSIZED_WORDS.to_le_bytes());
    assert!(decode_value(&header).is_err());
    header[..std::mem::size_of::<u32>()].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_value(&header).is_err());
}

#[test]
fn sliced_network_buffers_and_canonical_signature_material_preserve_binary_data() {
    let source = Value::Map(vec![(
        Value::from(TEXT_SENTINEL),
        Value::Data(vec![INVALID_UTF8]),
    )]);
    let bytes = encode(&source).unwrap();
    let mut unaligned = vec![u8::default(); UNALIGNED_OFFSET];
    unaligned.extend_from_slice(&bytes);
    let decoded: Value = decode(&unaligned[UNALIGNED_OFFSET..]).unwrap();
    assert_eq!(decoded, source);
    assert_eq!(canonical(&source).unwrap(), canonical(&decoded).unwrap());
    assert_ne!(canonical(&Value::Null).unwrap(), canonical(&false).unwrap());
}

#[test]
fn malformed_utf8_and_business_specific_depth_limits_fail_closed() {
    let mut bytes = encode(TEXT_SENTINEL).unwrap();
    let offset = bytes
        .windows(TEXT_SENTINEL.len())
        .position(|bytes| bytes == TEXT_SENTINEL.as_bytes())
        .unwrap();
    bytes[offset] = INVALID_UTF8;
    assert!(matches!(decode_value(&bytes), Err(ValueError::Utf8)));
    let source = Value::List(vec![Value::List(vec![Value::Null])]);
    let bytes = encode_value(&source).unwrap();
    assert!(matches!(
        decode_with_depth::<Value>(&bytes, CHILD_DEPTH),
        Err(ValueError::Depth)
    ));
    assert_eq!(
        decode_with_depth::<Value>(&bytes, CHILD_DEPTH + CHILD_DEPTH).unwrap(),
        source
    );
}

#[test]
fn outbound_byte_budgets_include_the_capnp_document_overhead() {
    let source = Value::Data(vec![u8::default(); yolo_capnp_codec::MAX_MESSAGE_BYTES]);
    assert!(matches!(encode_value(&source), Err(ValueError::Limit)));
    assert!(matches!(canonical(&source), Err(ValueError::Limit)));
    let oversized = Value::Data(vec![
        u8::default();
        yolo_capnp_codec::MAX_MESSAGE_BYTES + CHILD_DEPTH
    ]);
    assert!(matches!(encode_value(&oversized), Err(ValueError::Limit)));
    assert!(matches!(
        decode_value(&vec![
            u8::default();
            yolo_capnp_codec::MAX_MESSAGE_BYTES + CHILD_DEPTH
        ]),
        Err(ValueError::Limit)
    ));
}
