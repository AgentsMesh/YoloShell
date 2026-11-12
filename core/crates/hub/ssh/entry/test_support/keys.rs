use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8};
use std::sync::Arc;
use yolo_hub_client_access_protocol::ED25519_PUBLIC_KEY_BYTES;

pub fn keys() -> ([u8; ED25519_PUBLIC_KEY_BYTES], Arc<PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    (
        signing.public_key().as_ref().try_into().unwrap(),
        Arc::new(decode_pkcs8(document.as_ref(), None).unwrap()),
    )
}
