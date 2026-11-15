use crate::{AuditEntry, constants::*};
use std::collections::BTreeMap;
use std::fmt;
use yolo_hub_delegation::{ED25519_PUBLIC_KEY_BYTES, PeerIdentity};
use yolo_hub_identity::HubId;

/// Expected Hub identities for an authenticated offline merge. Every accepted
/// record must be signed by the Hub it names as its destination.
pub struct AuditTrust {
    identities: BTreeMap<HubId, [u8; ED25519_PUBLIC_KEY_BYTES]>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditTrustError {
    Invalid,
    Duplicate,
    Capacity,
}

impl fmt::Display for AuditTrustError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(TRUST_INVALID)
    }
}

impl std::error::Error for AuditTrustError {}

impl AuditTrust {
    pub fn parse(values: &[String]) -> Result<Self, AuditTrustError> {
        if values.len() > MAX_TRUSTED_AUDIT_SIGNERS {
            return Err(AuditTrustError::Capacity);
        }
        let mut identities = BTreeMap::new();
        for value in values {
            let (hub, key) = value
                .split_once(TRUST_SEPARATOR)
                .ok_or(AuditTrustError::Invalid)?;
            let hub = hub.parse::<HubId>().map_err(|_| AuditTrustError::Invalid)?;
            let key = decode_public_key(key).ok_or(AuditTrustError::Invalid)?;
            PeerIdentity::new(hub, key).map_err(|_| AuditTrustError::Invalid)?;
            if identities.insert(hub, key).is_some() {
                return Err(AuditTrustError::Duplicate);
            }
        }
        Ok(Self { identities })
    }

    pub fn accepts(&self, entry: &AuditEntry) -> bool {
        entry.authenticated()
            && entry
                .signer
                .is_some_and(|signer| self.identities.get(&signer.hub()) == Some(signer.key()))
    }
}

pub fn public_key_hex(identity: &PeerIdentity) -> String {
    let mut text = String::with_capacity(ED25519_PUBLIC_KEY_BYTES * HEX_DIGITS_PER_BYTE);
    for byte in identity.key() {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn decode_public_key(value: &str) -> Option<[u8; ED25519_PUBLIC_KEY_BYTES]> {
    if value.len() != ED25519_PUBLIC_KEY_BYTES * HEX_DIGITS_PER_BYTE {
        return None;
    }
    let mut key = [0_u8; ED25519_PUBLIC_KEY_BYTES];
    for (index, byte) in key.iter_mut().enumerate() {
        let start = index.checked_mul(HEX_DIGITS_PER_BYTE)?;
        let end = start.checked_add(HEX_DIGITS_PER_BYTE)?;
        *byte = u8::from_str_radix(value.get(start..end)?, 16).ok()?;
    }
    Some(key)
}
