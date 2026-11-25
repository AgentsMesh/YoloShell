use crate::support::*;
use serde::{Serialize, Serializer};
use yolo_hub_transport::*;

const INVALID_MARKER: u8 = 0xc1;
const EXTRA_VALUE: u8 = 0;
const FRAME_HEADROOM_BYTES: usize = 8 * 1024;
const SERIALIZE_FAILURE: &str = "injected serializer error";

#[test]
fn binary_message_roundtrips_with_an_exact_big_endian_header() {
    let expected = message();
    let frame = encode(&expected).unwrap();
    let len = u32::from_be_bytes(frame[..FRAME_LENGTH_BYTES].try_into().unwrap()) as usize;
    assert_eq!(len, frame.len() - FRAME_LENGTH_BYTES);
    assert_eq!(
        decode::<Message>(&frame[FRAME_LENGTH_BYTES..]).unwrap(),
        expected
    );
}

#[test]
fn malformed_empty_oversized_and_trailing_payloads_are_rejected() {
    assert!(matches!(
        decode::<Message>(&[]),
        Err(TransportError::EmptyFrame)
    ));
    assert!(matches!(
        decode::<Message>(&[INVALID_MARKER]),
        Err(TransportError::Decode(_))
    ));
    assert!(matches!(
        decode::<Message>(&vec![EXTRA_VALUE; MAX_FRAME_BYTES + 1]),
        Err(TransportError::FrameTooLarge)
    ));
    let mut frame = encode(&message()).unwrap();
    frame.push(EXTRA_VALUE);
    assert!(matches!(
        decode::<Message>(&frame[FRAME_LENGTH_BYTES..]),
        Err(TransportError::TrailingBytes)
    ));
}

#[test]
fn output_limit_is_checked_while_encoding_not_after_allocating_unbounded_output() {
    let data =
        serde_bytes::ByteBuf::from(vec![EXTRA_VALUE; MAX_FRAME_BYTES - FRAME_HEADROOM_BYTES]);
    let encoded = encode(&data).unwrap();
    assert!(encoded.len() <= MAX_FRAME_BYTES + FRAME_LENGTH_BYTES);
    assert_eq!(
        decode::<serde_bytes::ByteBuf>(&encoded[FRAME_LENGTH_BYTES..]).unwrap(),
        data
    );
    let too_large = serde_bytes::ByteBuf::from(vec![EXTRA_VALUE; MAX_FRAME_BYTES]);
    assert!(matches!(
        encode(&too_large),
        Err(TransportError::FrameTooLarge)
    ));
}

struct Rejected;
impl Serialize for Rejected {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom(SERIALIZE_FAILURE))
    }
}

#[test]
fn serializer_failure_remains_distinct_from_payload_size_failure() {
    let error = encode(&Rejected).unwrap_err();
    assert!(matches!(error, TransportError::Encode(_)));
    assert!(std::error::Error::source(&error).is_some());
    assert!(error.to_string().contains(SERIALIZE_FAILURE));
    for error in [
        TransportError::EmptyFrame,
        TransportError::FrameTooLarge,
        TransportError::TrailingBytes,
        TransportError::TruncatedFrame,
        TransportError::Deadline,
        TransportError::Poisoned,
    ] {
        assert!(!error.to_string().is_empty());
        assert!(std::error::Error::source(&error).is_none());
    }
}
