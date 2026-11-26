use crate::{FRAME_LENGTH_BYTES, MAX_FRAME_BYTES, TransportError};
use serde::{Serialize, de::DeserializeOwned};

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, TransportError> {
    let payload = yolo_capnp_codec::encode(value).map_err(|error| match error {
        yolo_capnp_codec::ValueError::Limit => TransportError::FrameTooLarge,
        other => TransportError::Encode(other),
    })?;
    if payload.len() > MAX_FRAME_BYTES {
        return Err(TransportError::FrameTooLarge);
    }
    let mut frame = Vec::with_capacity(FRAME_LENGTH_BYTES + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend(payload);
    Ok(frame)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, TransportError> {
    if bytes.is_empty() {
        return Err(TransportError::EmptyFrame);
    }
    if bytes.len() > MAX_FRAME_BYTES {
        return Err(TransportError::FrameTooLarge);
    }
    yolo_capnp_codec::decode(bytes).map_err(|error| match error {
        yolo_capnp_codec::ValueError::Trailing => TransportError::TrailingBytes,
        other => TransportError::Decode(other),
    })
}
