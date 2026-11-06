use super::constants::*;
use ring::signature::{Ed25519KeyPair, KeyPair};
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_identity::HubId;

pub fn identity() -> PeerIdentity {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    PeerIdentity::new(
        HubId::new(),
        signing.public_key().as_ref().try_into().unwrap(),
    )
    .unwrap()
}
pub fn address() -> PeerAddress {
    PeerAddress::ssh(PROFILE, SOCKET.into()).unwrap()
}
pub fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}
