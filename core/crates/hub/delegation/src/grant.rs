use crate::{DelegationError, PeerIdentity, Signer, constants::*};
use ring::{digest, signature};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_routing::Rights;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrantClaims {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub grant_id: Uuid,
    pub issuer: PeerIdentity,
    pub recipient: PeerIdentity,
    pub origin: PeerIdentity,
    pub policy_epoch: u64,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
    pub remaining_reexports: u8,
    pub rights: Rights,
    pub parent: Option<[u8; digest::SHA256_OUTPUT_LEN]>,
}

impl GrantClaims {
    pub fn validate(&self) -> Result<(), DelegationError> {
        if self.grant_id.is_nil()
            || self.issuer.hub() == self.recipient.hub()
            || self.origin.hub() == self.recipient.hub()
            || self.rights.is_empty()
            || self.policy_epoch < FIRST_POLICY_EPOCH
            || self.expires_at_ms <= self.not_before_ms
            || self.expires_at_ms - self.not_before_ms > MAX_GRANT_LIFETIME_MS
            || self.remaining_reexports > MAX_DELEGATION_DEPTH
        {
            return Err(DelegationError::Claims);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SignedGrantFields", into = "SignedGrantFields")]
pub struct SignedGrant(SignedGrantFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedGrantFields {
    pub claims: GrantClaims,
    pub signature: Vec<u8>,
}

#[derive(Serialize)]
struct Signable<'a> {
    domain: &'static str,
    version: u16,
    claims: &'a GrantClaims,
}

impl SignedGrant {
    pub fn issue(claims: GrantClaims, signer: &impl Signer) -> Result<Self, DelegationError> {
        claims.validate()?;
        if signer.identity() != claims.issuer {
            return Err(DelegationError::Identity);
        }
        let signature = signer.sign(&signing_bytes(&claims)?)?.to_vec();
        SignedGrantFields { claims, signature }.try_into()
    }
    pub fn fields(&self) -> &SignedGrantFields {
        &self.0
    }
    pub fn verify(&self) -> Result<(), DelegationError> {
        self.0.claims.validate()?;
        signature::UnparsedPublicKey::new(&signature::ED25519, self.0.claims.issuer.key())
            .verify(&signing_bytes(&self.0.claims)?, &self.0.signature)
            .map_err(|_| DelegationError::Signature)
    }
    pub fn fingerprint(&self) -> Result<[u8; digest::SHA256_OUTPUT_LEN], DelegationError> {
        let bytes = yolo_capnp_codec::canonical(self).map_err(|_| DelegationError::Signing)?;
        let mut hash = digest::Context::new(&digest::SHA256);
        hash.update(GRANT_DIGEST_DOMAIN);
        hash.update(&bytes);
        hash.finish()
            .as_ref()
            .try_into()
            .map_err(|_| DelegationError::Signing)
    }
}

fn signing_bytes(claims: &GrantClaims) -> Result<Vec<u8>, DelegationError> {
    yolo_capnp_codec::canonical(&Signable {
        domain: GRANT_SIGNATURE_DOMAIN,
        version: DELEGATION_SCHEMA,
        claims,
    })
    .map_err(|_| DelegationError::Signing)
}

impl TryFrom<SignedGrantFields> for SignedGrant {
    type Error = DelegationError;
    fn try_from(fields: SignedGrantFields) -> Result<Self, Self::Error> {
        if fields.signature.len() != ED25519_SIGNATURE_BYTES {
            return Err(DelegationError::Signature);
        }
        let grant = Self(fields);
        grant.verify()?;
        Ok(grant)
    }
}
impl From<SignedGrant> for SignedGrantFields {
    fn from(value: SignedGrant) -> Self {
        value.0
    }
}
