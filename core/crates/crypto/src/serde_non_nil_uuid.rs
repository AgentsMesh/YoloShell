use serde::{Deserializer, Serializer};
use uuid::Uuid;

const NIL_UUID_ERROR: &str = "UUID must not be nil";

pub fn serialize<S>(value: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    crate::serde_uuid::serialize(value, serializer)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    let value = crate::serde_uuid::deserialize(deserializer)?;
    if value.is_nil() {
        return Err(serde::de::Error::custom(NIL_UUID_ERROR));
    }
    Ok(value)
}
