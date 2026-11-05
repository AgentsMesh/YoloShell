use crate::constants::*;
use crate::model::{InviteClaims, InviteRequest};
use crate::InviteError;
use ring::signature;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use yolo_hub_delegation::{ED25519_SIGNATURE_BYTES, Signer};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SignedInviteFields", into = "SignedInviteFields")]
pub struct SignedInvite(SignedInviteFields);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedInviteFields {
    #[serde(default = "legacy_version")]
    pub version: u16,
    pub claims: InviteClaims,
    pub signature: Vec<u8>,
}

fn legacy_version() -> u16 {
    LEGACY_SIGNATURE_VERSION
}

#[derive(Serialize)]
struct Signable<'a> {
    domain: &'static str,
    version: u16,
    claims: &'a InviteClaims,
}

impl SignedInvite {
    pub fn issue(claims: InviteClaims, signer: &impl Signer) -> Result<Self, InviteError> {
        claims.validate()?;
        if signer.identity() != claims.issuer {
            return Err(InviteError::Issuer);
        }
        let version = if claims.dial.is_some() {
            LEGACY_SIGNATURE_VERSION
        } else {
            SIGNATURE_VERSION
        };
        let signature = signer
            .sign(&signing_bytes(version, &claims)?)
            .map_err(|_| InviteError::Signing)?
            .to_vec();
        SignedInviteFields {
            version,
            claims,
            signature,
        }
        .try_into()
    }

    pub fn claims(&self) -> &InviteClaims {
        &self.0.claims
    }

    pub fn version(&self) -> u16 {
        self.0.version
    }

    pub fn signature(&self) -> &[u8] {
        &self.0.signature
    }

    pub fn verify_at(&self, now_ms: u64) -> Result<(), InviteError> {
        self.0.claims.verify_at(now_ms)?;
        self.verify_signature()
    }

    fn verify_signature(&self) -> Result<(), InviteError> {
        signature::UnparsedPublicKey::new(&signature::ED25519, self.0.claims.issuer.key())
            .verify(
                &signing_bytes(self.0.version, &self.0.claims)?,
                &self.0.signature,
            )
            .map_err(|_| InviteError::Signature)
    }
}

/// Mints an invite for `request`; the caller owns the invite ID and clock.
pub fn mint(
    request: &InviteRequest,
    invite_id: Uuid,
    now_ms: u64,
    signer: &impl Signer,
) -> Result<SignedInvite, InviteError> {
    request.validate()?;
    let expires_at_ms = now_ms
        .checked_add(request.lifetime_ms)
        .ok_or(InviteError::Claims)?;
    SignedInvite::issue(
        InviteClaims {
            invite_id,
            issuer: signer.identity(),
            dial: request.dial.clone(),
            not_before_ms: now_ms,
            expires_at_ms,
            policy: request.policy,
        },
        signer,
    )
}

fn signing_bytes(version: u16, claims: &InviteClaims) -> Result<Vec<u8>, InviteError> {
    yolo_capnp_codec::canonical(&Signable {
        domain: SIGNATURE_DOMAIN,
        version,
        claims,
    })
    .map_err(|_| InviteError::Signing)
}

impl TryFrom<SignedInviteFields> for SignedInvite {
    type Error = InviteError;

    fn try_from(fields: SignedInviteFields) -> Result<Self, Self::Error> {
        fields.claims.validate()?;
        match (fields.version, fields.claims.dial.is_some()) {
            (LEGACY_SIGNATURE_VERSION, true) | (SIGNATURE_VERSION, false) => {}
            _ => return Err(InviteError::Claims),
        }
        if fields.signature.len() != ED25519_SIGNATURE_BYTES {
            return Err(InviteError::Signature);
        }
        let invite = Self(fields);
        invite.verify_signature()?;
        Ok(invite)
    }
}

impl From<SignedInvite> for SignedInviteFields {
    fn from(value: SignedInvite) -> Self {
        value.0
    }
}
