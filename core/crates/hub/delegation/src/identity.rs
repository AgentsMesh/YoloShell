use crate::{DelegationError, ED25519_PUBLIC_KEY_BYTES, ED25519_SIGNATURE_BYTES};
use ring::signature;
use serde::{Deserialize, Serialize};
use yolo_hub_identity::HubId;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "IdentityFields", into = "IdentityFields")]
pub struct PeerIdentity(IdentityFields);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityFields {
    pub hub: HubId,
    pub public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

impl PeerIdentity {
    pub fn new(
        hub: HubId,
        public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
    ) -> Result<Self, DelegationError> {
        IdentityFields { hub, public_key }.try_into()
    }
    pub fn hub(self) -> HubId {
        self.0.hub
    }
    pub fn key(&self) -> &[u8; ED25519_PUBLIC_KEY_BYTES] {
        &self.0.public_key
    }

    /// Verifies a signature made by the Hub key bound to this identity.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), DelegationError> {
        if signature.len() != ED25519_SIGNATURE_BYTES {
            return Err(DelegationError::Signature);
        }
        signature::UnparsedPublicKey::new(&signature::ED25519, self.key())
            .verify(message, signature)
            .map_err(|_| DelegationError::Signature)
    }
}

impl TryFrom<IdentityFields> for PeerIdentity {
    type Error = DelegationError;
    fn try_from(fields: IdentityFields) -> Result<Self, Self::Error> {
        if fields.public_key.iter().all(|byte| *byte == u8::default()) {
            return Err(DelegationError::Identity);
        }
        Ok(Self(fields))
    }
}
impl From<PeerIdentity> for IdentityFields {
    fn from(value: PeerIdentity) -> Self {
        value.0
    }
}

pub trait Signer {
    fn identity(&self) -> PeerIdentity;
    fn sign(&self, message: &[u8]) -> Result<[u8; ED25519_SIGNATURE_BYTES], DelegationError>;
}
