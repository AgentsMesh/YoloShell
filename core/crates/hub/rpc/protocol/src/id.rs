use serde::{Deserialize, Deserializer, Serializer, de::Error};
use uuid::Uuid;
use yolo_hub_terminal_protocol::PublicError;

pub(crate) fn deserialize<'de, D: Deserializer<'de>>(input: D) -> Result<Uuid, D::Error> {
    let value = String::deserialize(input)?;
    yolo_crypto::serde_uuid::parse_canonical(&value)
        .filter(|id| !id.is_nil())
        .ok_or_else(|| D::Error::custom(PublicError::InvalidRequest))
}

pub(crate) fn serialize<S: Serializer>(value: &Uuid, output: S) -> Result<S::Ok, S::Error> {
    if value.is_nil() {
        return Err(serde::ser::Error::custom(PublicError::InvalidRequest));
    }
    output.serialize_str(&value.hyphenated().to_string())
}
