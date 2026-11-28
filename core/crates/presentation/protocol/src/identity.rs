use crate::ProtocolError;
use serde::{Deserialize, Deserializer, Serializer};
use uuid::Uuid;

pub(crate) fn require(id: Uuid) -> Result<(), ProtocolError> {
    (!id.is_nil()).then_some(()).ok_or(ProtocolError::InvalidId)
}
pub fn serialize<S: Serializer>(id: &Uuid, serializer: S) -> Result<S::Ok, S::Error> {
    require(*id).map_err(serde::ser::Error::custom)?;
    serializer.serialize_str(&id.to_string())
}
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Uuid, D::Error> {
    let text = String::deserialize(deserializer)?;
    let id = text.parse::<Uuid>().map_err(serde::de::Error::custom)?;
    require(id).map_err(serde::de::Error::custom)?;
    if id.to_string() != text {
        return Err(serde::de::Error::custom(ProtocolError::InvalidId));
    }
    Ok(id)
}
