use ring::signature::{Ed25519KeyPair, KeyPair};
use yolo_hub_delegation::*;
use yolo_hub_identity::HubId;
use yolo_hub_routing::{Right, Rights};

pub const BEFORE: u64 = 1_000;
pub const NOW: u64 = 2_000;
pub const EXPIRES: u64 = 3_000;
pub const STEP: u64 = 1;
pub const REEXPORT_DEPTH: u8 = 2;
pub const INJECTED: &str = "injected";

pub struct Identity {
    pub identity: PeerIdentity,
    key: Ed25519KeyPair,
}
impl Identity {
    pub fn new() -> Self {
        let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
        let key = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
        let identity =
            PeerIdentity::new(HubId::new(), key.public_key().as_ref().try_into().unwrap()).unwrap();
        Self { identity, key }
    }
}
impl Signer for Identity {
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
pub fn rights() -> Rights {
    [Right::Observe, Right::TerminalRead, Right::TerminalWrite]
        .into_iter()
        .collect()
}
pub fn observe() -> Rights {
    [Right::Observe].into_iter().collect()
}
pub fn root_claims(origin: &Identity, recipient: &Identity) -> GrantClaims {
    GrantClaims {
        grant_id: uuid::Uuid::new_v4(),
        issuer: origin.identity,
        recipient: recipient.identity,
        origin: origin.identity,
        policy_epoch: FIRST_POLICY_EPOCH,
        not_before_ms: BEFORE,
        expires_at_ms: EXPIRES,
        remaining_reexports: REEXPORT_DEPTH,
        rights: rights(),
        parent: None,
    }
}
pub fn child_claims(parent: &SignedGrant, recipient: &Identity) -> GrantClaims {
    let mut claims = parent.fields().claims.clone();
    claims.grant_id = uuid::Uuid::new_v4();
    claims.issuer = claims.recipient;
    claims.recipient = recipient.identity;
    claims.parent = Some(parent.fingerprint().unwrap());
    claims.remaining_reexports = claims.remaining_reexports.saturating_sub(1);
    claims
}
