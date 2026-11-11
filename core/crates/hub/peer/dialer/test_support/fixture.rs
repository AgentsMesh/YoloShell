use super::constants::*;
use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8};
use std::{num::NonZeroU16, sync::Arc};
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_identity::HubId;

pub fn keys() -> (PeerIdentity, Arc<PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    let identity = PeerIdentity::new(
        HubId::new(),
        signing.public_key().as_ref().try_into().unwrap(),
    )
    .unwrap();
    (
        identity,
        Arc::new(decode_pkcs8(document.as_ref(), None).unwrap()),
    )
}
pub fn address() -> PeerAddress {
    PeerAddress::tcp(HOST, NonZeroU16::new(PORT).unwrap())
}
pub fn deadline() -> Instant {
    Instant::now() + TIMEOUT
}
