use crate::constants::*;
use anyhow::{Result, ensure};
#[cfg(any(test, feature = "yolo-test-support"))]
use ring::signature::Ed25519KeyPair;
use ring::signature::{ED25519, UnparsedPublicKey};

const HEX_RADIX: u8 = 16;
const DECIMAL_DIGIT_LIMIT: u8 = 10;
const ASCII_ZERO: u8 = b'0';
const ASCII_LOWER_A: u8 = b'a';
const ASCII_LOWER_F: u8 = b'f';
#[cfg(any(test, feature = "yolo-test-support"))]
const HEX_DIGIT_COUNT: usize = 16;

pub(crate) fn verify_signature_with_key(
    payload: &[u8],
    encoded: &str,
    public_key: &[u8],
) -> Result<()> {
    ensure!(
        public_key.len() == ED25519_PUBLIC_KEY_BYTES,
        RECEIPT_SIGNATURE_ERROR
    );
    let signature = decode_hex(encoded)?;
    let key = UnparsedPublicKey::new(&ED25519, public_key);
    key.verify(payload, &signature)
        .map_err(|_| anyhow::anyhow!(RECEIPT_SIGNATURE_ERROR))
}

pub(crate) fn decode_hex(value: &str) -> Result<Vec<u8>> {
    ensure!(
        value.len() == SIGNATURE_HEX_CHARACTERS,
        RECEIPT_SIGNATURE_ERROR
    );
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(ED25519_SIGNATURE_BYTES);
    for pair in bytes.chunks_exact(HEX_CHARACTERS_PER_BYTE) {
        let high = hex_digit(pair[0])?;
        let low = hex_digit(pair[1])?;
        decoded.push(high * HEX_RADIX + low);
    }
    Ok(decoded)
}

fn hex_digit(value: u8) -> Result<u8> {
    if value.is_ascii_digit() {
        return Ok(value - ASCII_ZERO);
    }
    ensure!(
        (ASCII_LOWER_A..=ASCII_LOWER_F).contains(&value),
        RECEIPT_SIGNATURE_ERROR
    );
    Ok(value - ASCII_LOWER_A + DECIMAL_DIGIT_LIMIT)
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub(crate) fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; HEX_DIGIT_COUNT] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * HEX_CHARACTERS_PER_BYTE);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & (HEX_RADIX - 1)) as usize] as char);
    }
    encoded
}

#[cfg(any(test, feature = "yolo-test-support"))]
pub(crate) fn sign_with_test_key(payload: &[u8], pair: &Ed25519KeyPair) -> String {
    encode_hex(pair.sign(payload).as_ref())
}
