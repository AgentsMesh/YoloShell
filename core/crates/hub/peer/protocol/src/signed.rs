use crate::{PeerError, constants::*};
use ring::{digest, signature};
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::{ED25519_SIGNATURE_BYTES, PeerIdentity, Signer};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Purpose {
    Catalogue,
    Call,
    Response,
    Terminal,
    Audit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signed<T> {
    pub signer: PeerIdentity,
    pub purpose: Purpose,
    pub payload: T,
    pub signature: Vec<u8>,
}

#[derive(Serialize)]
struct Document<'a, T> {
    domain: &'static str,
    purpose: Purpose,
    signer: PeerIdentity,
    payload: &'a T,
}

impl<T: Serialize> Signed<T> {
    pub fn new(payload: T, purpose: Purpose, signer: &(impl Signer + ?Sized)) -> Result<Self, PeerError> {
        let mut value = Self {
            payload,
            purpose,
            signer: signer.identity(),
            signature: Vec::new(),
        };
        value.signature = signer.sign(&value.signing_bytes()?)?.to_vec();
        value.verify(purpose, signer.identity())?;
        Ok(value)
    }
    pub fn verify(&self, purpose: Purpose, signer: PeerIdentity) -> Result<(), PeerError> {
        if purpose != self.purpose
            || signer != self.signer
            || self.signature.len() != ED25519_SIGNATURE_BYTES
        {
            return Err(PeerError::Signature);
        }
        signature::UnparsedPublicKey::new(&signature::ED25519, signer.key())
            .verify(&self.signing_bytes()?, &self.signature)
            .map_err(|_| PeerError::Signature)
    }
    pub fn fingerprint(&self) -> Result<[u8; digest::SHA256_OUTPUT_LEN], PeerError> {
        let bytes = yolo_capnp_codec::canonical(self).map_err(|_| PeerError::Encoding)?;
        digest::digest(&digest::SHA256, &bytes)
            .as_ref()
            .try_into()
            .map_err(|_| PeerError::Encoding)
    }
    fn signing_bytes(&self) -> Result<Vec<u8>, PeerError> {
        yolo_capnp_codec::canonical(&Document {
            domain: SIGNATURE_DOMAIN,
            signer: self.signer,
            purpose: self.purpose,
            payload: &self.payload,
        })
        .map_err(|_| PeerError::Encoding)
    }
}
