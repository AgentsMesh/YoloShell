use crate::constants::*;
use crate::{InviteError, codec};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU16;
use uuid::Uuid;
use yolo_hub_delegation::PeerIdentity;

/// Parameters for a Hub-local mint request. The Hub supplies the issuer
/// identity, invite ID and time; callers cannot choose them.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InviteRequest {
    pub dial: InviteDial,
    pub lifetime_ms: u64,
    pub policy: InvitePolicy,
}

/// A minted code plus the values a client displays and correlates.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MintedInvite {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub invite_id: Uuid,
    pub code: String,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InviteClaims {
    #[serde(with = "yolo_crypto::serde_non_nil_uuid")]
    pub invite_id: Uuid,
    pub issuer: PeerIdentity,
    pub dial: InviteDial,
    pub not_before_ms: u64,
    pub expires_at_ms: u64,
    pub policy: InvitePolicy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InviteDial {
    pub host: String,
    pub port: NonZeroU16,
}

/// What the inviting Hub grants the joining Hub, and whether it imports it.
/// The joining side mirrors these directions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InvitePolicy {
    pub export_local: bool,
    pub import_peer: bool,
}

impl InviteRequest {
    pub fn validate(&self) -> Result<(), InviteError> {
        self.dial.validate()?;
        valid_lifetime(self.lifetime_ms)?;
        valid_policy(self.policy)
    }
}

impl MintedInvite {
    /// Re-decodes the signed code so a reply cannot disagree with itself.
    pub fn validate(&self) -> Result<(), InviteError> {
        let invite = codec::decode(&self.code)?;
        if invite.claims().invite_id != self.invite_id
            || invite.claims().expires_at_ms != self.expires_at_ms
        {
            return Err(InviteError::Code);
        }
        Ok(())
    }
}

impl InviteDial {
    pub fn validate(&self) -> Result<(), InviteError> {
        if self.host.is_empty()
            || self.host.len() > MAX_HOST_BYTES
            || self
                .host
                .chars()
                .any(|character| character.is_whitespace() || character.is_control())
        {
            return Err(InviteError::Claims);
        }
        Ok(())
    }
}

impl InviteClaims {
    pub fn validate(&self) -> Result<(), InviteError> {
        if self.invite_id.is_nil() || self.issuer.hub().as_uuid().is_nil() {
            return Err(InviteError::Claims);
        }
        self.dial.validate()?;
        valid_policy(self.policy)?;
        if self.not_before_ms >= self.expires_at_ms {
            return Err(InviteError::Claims);
        }
        valid_lifetime(self.expires_at_ms - self.not_before_ms)
    }

    pub fn verify_at(&self, now_ms: u64) -> Result<(), InviteError> {
        self.validate()?;
        if now_ms.saturating_add(CLOCK_SKEW_MS) < self.not_before_ms
            || now_ms >= self.expires_at_ms
        {
            return Err(InviteError::Expired);
        }
        Ok(())
    }
}

pub(crate) fn valid_lifetime(lifetime_ms: u64) -> Result<(), InviteError> {
    if !(MIN_LIFETIME_MS..=MAX_LIFETIME_MS).contains(&lifetime_ms) {
        return Err(InviteError::Claims);
    }
    Ok(())
}

pub(crate) fn valid_policy(policy: InvitePolicy) -> Result<(), InviteError> {
    if !policy.export_local && !policy.import_peer {
        return Err(InviteError::Claims);
    }
    Ok(())
}
