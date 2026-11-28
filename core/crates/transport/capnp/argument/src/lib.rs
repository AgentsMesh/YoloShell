//! Bounded textual carriage of Cap'n Proto documents in an executable argument.
use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use serde::{Serialize, de::DeserializeOwned};
use yolo_capnp_codec::ValueError;

const ENCODED_GROUP_BYTES: usize = 4;
const DECODED_GROUP_BYTES: usize = 3;

pub fn encode(value: &impl Serialize, limit: usize) -> Result<String, ValueError> {
    let bytes = yolo_capnp_codec::encode(value)?;
    if bytes.len() > limit {
        return Err(ValueError::Limit);
    }
    Ok(STANDARD_NO_PAD.encode(bytes))
}

pub fn decode<T: DeserializeOwned>(value: &str, limit: usize) -> Result<T, ValueError> {
    let maximum = limit
        .div_ceil(DECODED_GROUP_BYTES)
        .checked_mul(ENCODED_GROUP_BYTES)
        .ok_or(ValueError::Limit)?;
    if value.len() > maximum {
        return Err(ValueError::Limit);
    }
    let bytes = STANDARD_NO_PAD
        .decode(value)
        .map_err(|error| ValueError::Invalid(error.to_string()))?;
    if bytes.len() > limit {
        return Err(ValueError::Limit);
    }
    yolo_capnp_codec::decode(&bytes)
}
