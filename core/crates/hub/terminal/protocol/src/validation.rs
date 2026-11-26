use serde::{Deserialize, Deserializer, Serializer, de::Error};
use uuid::Uuid;

use crate::PublicError;

pub(crate) fn request_id(value: Uuid) -> Result<(), PublicError> {
    if value.is_nil() {
        Err(PublicError::InvalidRequest)
    } else {
        Ok(())
    }
}

pub(crate) fn deserialize_id<'de, D: Deserializer<'de>>(input: D) -> Result<Uuid, D::Error> {
    let value = String::deserialize(input)?;
    let parsed = Uuid::parse_str(&value).map_err(D::Error::custom)?;
    if parsed.to_string() != value || parsed.is_nil() {
        return Err(D::Error::custom(PublicError::InvalidRequest));
    }
    Ok(parsed)
}

pub(crate) fn serialize_id<S: Serializer>(value: &Uuid, output: S) -> Result<S::Ok, S::Error> {
    output.serialize_str(&value.to_string())
}

pub(crate) fn deserialize_sequence<'de, D: Deserializer<'de>>(input: D) -> Result<u64, D::Error> {
    let value = u64::deserialize(input)?;
    if value == u64::MAX {
        return Err(D::Error::custom(PublicError::InvalidCursor));
    }
    Ok(value)
}

pub(crate) fn deserialize_generation<'de, D: Deserializer<'de>>(input: D) -> Result<u64, D::Error> {
    let value = u64::deserialize(input)?;
    if value < crate::FIRST_LEASE_GENERATION {
        return Err(D::Error::custom(PublicError::InvalidLease));
    }
    Ok(value)
}

pub(crate) fn metadata(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty() && value.len() <= maximum && !value.contains('\0')
}
