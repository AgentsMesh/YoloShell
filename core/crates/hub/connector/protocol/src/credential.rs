use crate::{ConnectorError, REDACTED_CREDENTIAL};
use serde::{Deserialize, Serialize};
use std::fmt;
use subtle::ConstantTimeEq;
use uuid::{Uuid, Variant, Version};

/// An enrollment secret, distinct from a terminal writer grant or peer identity.
#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ConnectorCredential(Uuid);

impl ConnectorCredential {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(value: Uuid) -> Result<Self, ConnectorError> {
        if value.get_variant() != Variant::RFC4122 || value.get_version() != Some(Version::Random) {
            return Err(ConnectorError::InvalidRequest);
        }
        Ok(Self(value))
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }

    pub fn matches(&self, other: &Self) -> bool {
        bool::from(self.0.as_bytes().ct_eq(other.0.as_bytes()))
    }
}

impl PartialEq for ConnectorCredential {
    fn eq(&self, other: &Self) -> bool {
        self.matches(other)
    }
}
impl Eq for ConnectorCredential {}

impl fmt::Debug for ConnectorCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(REDACTED_CREDENTIAL)
    }
}
impl fmt::Display for ConnectorCredential {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(REDACTED_CREDENTIAL)
    }
}

impl TryFrom<String> for ConnectorCredential {
    type Error = ConnectorError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let value = yolo_crypto::serde_uuid::parse_canonical(&value)
            .ok_or(ConnectorError::InvalidRequest)?;
        Self::from_uuid(value)
    }
}

impl From<ConnectorCredential> for String {
    fn from(value: ConnectorCredential) -> Self {
        value.0.to_string()
    }
}
