use serde::Serialize;
use serde::de::DeserializeOwned;
use yolo_capnp_value::Value;

#[path = "constants.rs"]
mod constants;
#[path = "envelope.rs"]
mod envelope;
#[path = "error.rs"]
mod error;
#[path = "framing.rs"]
mod framing;

pub use constants::*;
pub use envelope::{ErrorBody, EventEnvelope, RequestEnvelope, ResponseEnvelope};
pub use error::ProtocolError;
pub use framing::{
    OptionalFrameRead, read_frame, read_frame_blocking, read_optional_frame, write_frame,
    write_frame_blocking,
};

pub fn to_value<T: Serialize>(value: &T) -> Result<Value, ProtocolError> {
    yolo_capnp_value::to_value(value).map_err(ProtocolError::Encode)
}

pub fn from_value<T: DeserializeOwned>(value: Value) -> Result<T, ProtocolError> {
    yolo_capnp_value::from_value(value).map_err(ProtocolError::Decode)
}

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let payload = yolo_capnp_codec::encode(value).map_err(|error| match error {
        yolo_capnp_value::ValueError::Limit => ProtocolError::FrameTooLarge {
            length: MAX_FRAME_BYTES + 1,
        },
        other => ProtocolError::Encode(other),
    })?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: payload.len(),
        });
    }
    let mut frame = Vec::with_capacity(FRAME_LENGTH_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

pub fn decode<T: DeserializeOwned>(payload: &[u8]) -> Result<T, ProtocolError> {
    if payload.len() > MAX_FRAME_BYTES {
        return Err(ProtocolError::FrameTooLarge {
            length: payload.len(),
        });
    }
    yolo_capnp_codec::decode(payload).map_err(ProtocolError::Decode)
}

pub fn decode_with_depth<T: DeserializeOwned>(
    payload: &[u8],
    maximum: usize,
) -> Result<T, ProtocolError> {
    yolo_capnp_codec::decode_with_depth(payload, maximum).map_err(ProtocolError::Decode)
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/envelope_contract.rs"]
mod envelope_contract;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests_async.rs"]
mod tests_async;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests_envelope.rs"]
mod tests_envelope;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests_framing_branches.rs"]
mod tests_framing_branches;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests_optional_frame.rs"]
mod tests_optional_frame;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/tests_sync.rs"]
mod tests_sync;
