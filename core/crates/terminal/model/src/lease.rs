use serde::de::Error as DeserializeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;
use uuid::{Uuid, Variant, Version};

const REDACTED_CAPABILITY: &str = "[REDACTED]";
const INVALID_CAPABILITY: &str = "writer capability must be a non-nil UUID v4 string";

#[derive(Clone, PartialEq, Eq)]
pub struct WriterCapability(Uuid);

impl WriterCapability {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Debug for WriterCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("WriterCapability")
            .field(&REDACTED_CAPABILITY)
            .finish()
    }
}

impl Serialize for WriterCapability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0.hyphenated().to_string())
    }
}

impl<'de> Deserialize<'de> for WriterCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = String::deserialize(deserializer)?;
        let capability = Uuid::parse_str(&wire).map_err(D::Error::custom)?;
        if capability.is_nil()
            || capability.get_variant() != Variant::RFC4122
            || capability.get_version() != Some(Version::Random)
        {
            return Err(D::Error::custom(INVALID_CAPABILITY));
        }
        Ok(Self(capability))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterLease {
    pub holder_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controller_instance_id: Option<String>,
    pub generation: u64,
    pub expires_at_ms: i64,
    pub ttl_ms: u64,
    pub renew_after_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriterLeaseGrant {
    #[serde(flatten)]
    pub lease: WriterLease,
    pub writer_capability: WriterCapability,
}

impl Deref for WriterLeaseGrant {
    type Target = WriterLease;

    fn deref(&self) -> &Self::Target {
        &self.lease
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../unit_tests/lease_inline_tests.rs"]
mod tests;
