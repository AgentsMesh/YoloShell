use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::{Uuid, Variant, Version};

use crate::{CAPABILITY_DEBUG_NAME, PublicError, REDACTED_CAPABILITY};

#[derive(Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct SecretCapability(Uuid);

impl SecretCapability {
    pub fn generate() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(value: Uuid) -> Result<Self, PublicError> {
        if value.get_variant() != Variant::RFC4122 || value.get_version() != Some(Version::Random) {
            return Err(PublicError::InvalidCapability);
        }
        Ok(Self(value))
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl fmt::Debug for SecretCapability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple(CAPABILITY_DEBUG_NAME)
            .field(&REDACTED_CAPABILITY)
            .finish()
    }
}

impl TryFrom<String> for SecretCapability {
    type Error = PublicError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parsed = Uuid::parse_str(&value).map_err(|_| PublicError::InvalidCapability)?;
        if parsed.to_string() != value {
            return Err(PublicError::InvalidCapability);
        }
        Self::from_uuid(parsed)
    }
}

impl From<SecretCapability> for String {
    fn from(value: SecretCapability) -> Self {
        value.0.to_string()
    }
}
