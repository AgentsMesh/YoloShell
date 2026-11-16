use super::{
    AcceptanceEnvelope, AcceptancePayload, AcceptanceReceipt, canonical_envelope,
    canonical_payload, open_bounded,
};
use crate::constants::*;
use crate::crypto::sign_with_test_key;
use crate::record::AcceptanceRecord;
use anyhow::Result;
use ring::rand::SystemRandom;
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::path::Path;
use std::sync::OnceLock;

struct TestAuthority {
    pair: Ed25519KeyPair,
    public_key: Vec<u8>,
}

static TEST_AUTHORITY: OnceLock<TestAuthority> = OnceLock::new();

fn test_authority() -> &'static TestAuthority {
    TEST_AUTHORITY.get_or_init(|| {
        let document = Ed25519KeyPair::generate_pkcs8(&SystemRandom::new()).unwrap();
        let pair = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
        let public_key = pair.public_key().as_ref().to_vec();
        TestAuthority { pair, public_key }
    })
}

pub(crate) fn signed_for_test(
    record: AcceptanceRecord,
    pair: &ring::signature::Ed25519KeyPair,
) -> Result<Vec<u8>> {
    record.validate_shape()?;
    let payload = AcceptancePayload {
        schema: ACCEPTANCE_ENVELOPE_SCHEMA,
        authority: ACCEPTANCE_AUTHORITY.to_owned(),
        algorithm: ACCEPTANCE_ALGORITHM.to_owned(),
        record,
    };
    let signature = sign_with_test_key(&canonical_payload(&payload)?, pair);
    canonical_envelope(&AcceptanceEnvelope { payload, signature })
}

pub fn signed_with_test_authority(record: AcceptanceRecord) -> Result<Vec<u8>> {
    signed_for_test(record, &test_authority().pair)
}

pub fn test_public_key() -> &'static [u8] {
    &test_authority().public_key
}

pub fn from_bytes_with_test_key(bytes: &[u8], public_key: &[u8]) -> Result<AcceptanceReceipt> {
    AcceptanceReceipt::from_bytes_with_key(bytes, public_key)
}

pub fn open_with_test_key(path: &Path, public_key: &[u8]) -> Result<AcceptanceReceipt> {
    let source = open_bounded(path)?;
    let mut receipt = AcceptanceReceipt::from_bytes_with_key(source.bytes(), public_key)?;
    receipt.source = Some(source);
    Ok(receipt)
}
