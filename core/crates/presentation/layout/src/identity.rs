use serde::{Deserialize, Deserializer, Serializer};
use uuid::Uuid;

pub fn serialize<S: Serializer>(id: &Uuid, serializer: S) -> Result<S::Ok, S::Error> {
    if id.is_nil() {
        return Err(serde::ser::Error::custom(crate::LayoutError::InvalidId));
    }
    serializer.serialize_str(&id.to_string())
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Uuid, D::Error> {
    let text = String::deserialize(deserializer)?;
    let id = text.parse::<Uuid>().map_err(serde::de::Error::custom)?;
    if id.is_nil() || id.to_string() != text {
        return Err(serde::de::Error::custom(crate::LayoutError::InvalidId));
    }
    Ok(id)
}
