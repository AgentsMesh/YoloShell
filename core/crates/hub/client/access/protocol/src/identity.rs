use crate::{AccessError, ED25519_PUBLIC_KEY_BYTES, constants::EMPTY_PUBLIC_KEY};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "IdentityWire")]
pub struct ClientIdentity {
    #[serde(with = "yolo_crypto::serde_uuid")]
    pub client_id: Uuid,
    pub public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

impl ClientIdentity {
    pub fn validate(&self) -> Result<(), AccessError> {
        if self.client_id.is_nil() || self.public_key == EMPTY_PUBLIC_KEY {
            Err(AccessError::Identity)
        } else {
            Ok(())
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityWire {
    #[serde(with = "yolo_crypto::serde_uuid")]
    client_id: Uuid,
    public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

impl TryFrom<IdentityWire> for ClientIdentity {
    type Error = AccessError;

    fn try_from(wire: IdentityWire) -> Result<Self, Self::Error> {
        let identity = Self {
            client_id: wire.client_id,
            public_key: wire.public_key,
        };
        identity.validate()?;
        Ok(identity)
    }
}
