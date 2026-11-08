use serde::{Deserialize, Deserializer, Serializer};
use uuid::Uuid;

const CANONICAL_UUID_ERROR: &str = "UUID must use canonical lowercase hyphenated form";

pub fn serialize<S>(value: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.hyphenated().to_string())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    parse_canonical(&value).ok_or_else(|| serde::de::Error::custom(CANONICAL_UUID_ERROR))
}

pub fn parse_canonical(value: &str) -> Option<Uuid> {
    let parsed = Uuid::parse_str(value).ok()?;
    (parsed.hyphenated().to_string() == value).then_some(parsed)
}

pub mod option {
    use super::{CANONICAL_UUID_ERROR, Uuid, parse_canonical};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(value: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        value
            .map(|uuid| uuid.hyphenated().to_string())
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Uuid>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|value| {
                parse_canonical(&value)
                    .ok_or_else(|| serde::de::Error::custom(CANONICAL_UUID_ERROR))
            })
            .transpose()
    }
}
