use super::constants::*;
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, Ed25519KeyPair};
use russh::keys::{
    PrivateKey,
    pkcs8::decode_pkcs8,
    ssh_key::{Cipher, Kdf, LineEnding},
};

pub fn ed25519() -> PrivateKey {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    decode_pkcs8(document.as_ref(), None).unwrap()
}

pub fn p256() -> PrivateKey {
    let document = EcdsaKeyPair::generate_pkcs8(
        &ECDSA_P256_SHA256_ASN1_SIGNING,
        &ring::rand::SystemRandom::new(),
    )
    .unwrap();
    decode_pkcs8(document.as_ref(), None).unwrap()
}

pub fn encode(key: &PrivateKey) -> zeroize::Zeroizing<String> {
    key.to_openssh(LineEnding::LF).unwrap()
}

pub fn encrypted(key: &PrivateKey) -> PrivateKey {
    key.encrypt_with(
        Cipher::Aes256Ctr,
        Kdf::Bcrypt {
            salt: vec![SALT_VALUE; SALT_BYTES],
            rounds: ENCRYPTION_ROUNDS,
        },
        ENCRYPTION_CHECK,
        SECRET_MARKER,
    )
    .unwrap()
}
