use super::*;
use serde::Deserialize;
use serde::de::{DeserializeSeed, IntoDeserializer, SeqAccess};
use std::hash::{Hash, Hasher};

const TEST_HANDLE_KIND: i32 = 77;
const TEST_HANDLE_BYTE: u8 = 0x51;
const MAXIMUM_CONSUMED_ELEMENTS: usize = HANDLE_CAPACITY + 1;
const INVALID_KERNEL_HANDLE: i32 = 255;

#[test]
fn handle_bounds_are_enforced_at_construction_and_deserialization() {
    assert!(Handle::new(TEST_HANDLE_KIND, &[]).is_err());
    assert!(Handle::new(i32::default(), &[TEST_HANDLE_BYTE]).is_err());
    assert!(Handle::new(INVALID_KERNEL_HANDLE, &[TEST_HANDLE_BYTE]).is_err());
    assert!(Handle::new(TEST_HANDLE_KIND, &[TEST_HANDLE_BYTE; HANDLE_CAPACITY + 1]).is_err());
    let value = Handle::new(TEST_HANDLE_KIND, &[TEST_HANDLE_BYTE; HANDLE_CAPACITY]).unwrap();
    let wire = serde_json::to_value(value).unwrap();
    assert_eq!(serde_json::from_value::<Handle>(wire).unwrap(), value);
}

#[test]
fn malformed_handle_wire_rejects_wrong_container_and_empty_bytes() {
    for bytes in [
        serde_json::json!(true),
        serde_json::json!([]),
        serde_json::json!(vec![TEST_HANDLE_BYTE; MAXIMUM_CONSUMED_ELEMENTS]),
    ] {
        let input = serde_json::json!({"kind": TEST_HANDLE_KIND, "bytes": bytes});
        assert!(serde_json::from_value::<Handle>(input).is_err());
    }
}

#[test]
fn decoded_handles_have_canonical_equality_and_hash() {
    let value = Handle::new(TEST_HANDLE_KIND, &[TEST_HANDLE_BYTE]).unwrap();
    let decoded: Handle = serde_json::from_str(&serde_json::to_string(&value).unwrap()).unwrap();
    let hash = |value: Handle| {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut state);
        state.finish()
    };
    assert_eq!(decoded, value);
    assert_eq!(hash(decoded), hash(value));
}

#[test]
fn unbounded_input_sequence_is_rejected_after_one_bounded_buffer() {
    struct Infinite {
        consumed: usize,
    }
    impl<'de> SeqAccess<'de> for &mut Infinite {
        type Error = serde::de::value::Error;
        fn next_element_seed<T: DeserializeSeed<'de>>(
            &mut self,
            seed: T,
        ) -> Result<Option<T::Value>, Self::Error> {
            self.consumed += 1;
            assert!(self.consumed <= MAXIMUM_CONSUMED_ELEMENTS);
            seed.deserialize(TEST_HANDLE_BYTE.into_deserializer())
                .map(Some)
        }
    }
    let mut input = Infinite {
        consumed: usize::default(),
    };
    let result =
        crate::model::Bytes::deserialize(serde::de::value::SeqAccessDeserializer::new(&mut input));
    assert!(result.is_err());
    assert_eq!(input.consumed, MAXIMUM_CONSUMED_ELEMENTS);
}
