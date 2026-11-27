use std::sync::atomic::{AtomicUsize, Ordering};
use yolo_capnp_codec::{
    MAX_VALUE_DEPTH, Value, ValueError, decode_value_with_cleanup, encode_value,
};
use yolo_capnp_sensitive::{decode, erase};

const PRIVATE_KEY: &str = "test-private-key";
const PRIVATE_VALUE: &str = "test-private-value";
const INVALID_TEXT: &str = "malformed-final-node";
const INVALID_UTF8: u8 = 0xff;
const PRIVATE_FIELDS: usize = 2;
static SCRUBBED: AtomicUsize = AtomicUsize::new(0);

fn count_private(value: &Value) -> usize {
    match value {
        Value::Text(value) if value == PRIVATE_KEY || value == PRIVATE_VALUE => 1,
        Value::List(values) => values.iter().map(count_private).sum(),
        Value::Map(values) => values
            .iter()
            .map(|(key, value)| count_private(key) + count_private(value))
            .sum(),
        _ => 0,
    }
}
fn observe_erase(value: &mut Value) {
    SCRUBBED.fetch_add(count_private(value), Ordering::SeqCst);
    erase(value);
    assert_eq!(count_private(value), 0);
}

#[test]
fn malformed_tail_scrubs_accumulated_list_values_and_uncommitted_map_key() {
    let source = Value::Map(vec![(
        Value::from(PRIVATE_KEY),
        Value::List(vec![Value::from(PRIVATE_VALUE), Value::from(INVALID_TEXT)]),
    )]);
    let mut bytes = encode_value(&source).unwrap();
    let offset = bytes
        .windows(INVALID_TEXT.len())
        .position(|part| part == INVALID_TEXT.as_bytes())
        .unwrap();
    bytes[offset] = INVALID_UTF8;
    SCRUBBED.store(0, Ordering::SeqCst);
    assert!(matches!(
        decode_value_with_cleanup(&bytes, observe_erase),
        Err(ValueError::Utf8)
    ));
    assert_eq!(SCRUBBED.load(Ordering::SeqCst), PRIVATE_FIELDS);
    assert!(decode(&bytes).is_err());
}

#[test]
fn duplicate_normalization_preserves_float_and_composite_key_semantics() {
    let first = Value::Map(vec![
        (Value::from("b"), Value::Boolean(true)),
        (Value::from("a"), Value::Unsigned(1)),
    ]);
    let reverse = Value::Map(vec![
        (Value::from("a"), Value::Unsigned(1)),
        (Value::from("b"), Value::Boolean(true)),
    ]);
    for (left, right) in [
        (Value::Float(f64::NAN), Value::Float(-f64::NAN)),
        (Value::Float(0.0), Value::Float(-0.0)),
        (first, reverse),
    ] {
        let source = Value::Map(vec![
            (left, Value::from(PRIVATE_VALUE)),
            (right, Value::Null),
        ]);
        let bytes = encode_value(&source).unwrap();
        assert!(matches!(decode(&bytes), Err(ValueError::DuplicateKey)));
    }
}

#[test]
fn scrubbed_decode_retains_scalar_types_and_full_structural_depth() {
    let source = Value::List(vec![
        Value::Signed(-1),
        Value::Unsigned(u64::MAX),
        Value::Float(1.5),
        Value::Data(vec![INVALID_UTF8]),
    ]);
    assert_eq!(decode(&encode_value(&source).unwrap()).unwrap().0, source);
    let mut deepest = Value::from(PRIVATE_VALUE);
    for _ in 0..MAX_VALUE_DEPTH {
        deepest = Value::List(vec![deepest]);
    }
    assert_eq!(decode(&encode_value(&deepest).unwrap()).unwrap().0, deepest);
}
