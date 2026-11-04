use ring::{
    rand::SystemRandom,
    signature::{Ed25519KeyPair, KeyPair},
};
use std::num::NonZeroU16;
use uuid::Uuid;
use yolo_hub_delegation::{DelegationError, ED25519_SIGNATURE_BYTES, PeerIdentity, Signer};
use yolo_hub_enrollment_protocol::{InviteClaims, InviteDial, InvitePolicy, SignedInvite};
use yolo_hub_identity::HubId;

mod codec_tests;
mod expiry;
mod mint;

pub(crate) const HOST: &str = "192.168.139.211";
pub(crate) const NOW_MS: u64 = 1_700_000_000_000;
pub(crate) const LIFETIME_MS: u64 = 10 * 60 * 1000;

pub(crate) struct TestSigner {
    pub(crate) identity: PeerIdentity,
    key: Ed25519KeyPair,
}

impl TestSigner {
    pub(crate) fn new() -> Self {
        let pkcs8 = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let key = Ed25519KeyPair::from_pkcs8(pkcs8.as_ref()).unwrap();
        let public = key.public_key().as_ref().try_into().unwrap();
        Self {
            identity: PeerIdentity::new(HubId::new(), public).unwrap(),
            key,
        }
    }

    pub(crate) fn invite(&self, lifetime_ms: u64, policy: InvitePolicy) -> SignedInvite {
        SignedInvite::issue(claims(self, lifetime_ms, policy), self).unwrap()
    }
}

impl Signer for TestSigner {
    fn identity(&self) -> PeerIdentity {
        self.identity
    }

    fn sign(&self, message: &[u8]) -> Result<[u8; ED25519_SIGNATURE_BYTES], DelegationError> {
        self.key
            .sign(message)
            .as_ref()
            .try_into()
            .map_err(|_| DelegationError::Signing)
    }
}

pub(crate) fn claims(
    signer: &TestSigner,
    lifetime_ms: u64,
    policy: InvitePolicy,
) -> InviteClaims {
    InviteClaims {
        invite_id: Uuid::new_v4(),
        issuer: signer.identity,
        dial: InviteDial {
            host: HOST.into(),
            port: NonZeroU16::new(7400).unwrap(),
        },
        not_before_ms: NOW_MS,
        expires_at_ms: NOW_MS + lifetime_ms,
        policy,
    }
}

pub(crate) fn mutual() -> InvitePolicy {
    InvitePolicy {
        export_local: true,
        import_peer: true,
    }
}
