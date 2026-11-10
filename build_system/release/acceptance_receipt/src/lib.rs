use crate::constants::*;
use crate::io::{OpenReceiptFile, open_bounded, reject_duplicate_fields};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::Path;
use yolo_release_foundation::digest::bytes_sha256;

pub mod constants;
mod crypto;
mod expectation;
mod io;
mod metadata;
mod record;
mod record_validation;
mod time;

pub use constants::pending_manual_acceptance;
pub use expectation::AcceptanceExpectation;
pub use record::{
    AcceptanceCandidateBinding, AcceptanceCheckResult, AcceptancePlatform,
    AcceptanceProviderVersion, AcceptanceRecord,
};
pub use time::current_unix_seconds;

#[cfg(feature = "yolo-test-support")]
#[path = "../test_support/authority.rs"]
mod test_authority;
#[cfg(feature = "yolo-test-support")]
pub use test_authority::{
    from_bytes_with_test_key, open_with_test_key, signed_with_test_authority, test_public_key,
};

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/mod.rs"]
mod integration_tests;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AcceptancePayload {
    schema: u16,
    authority: String,
    algorithm: String,
    record: AcceptanceRecord,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct AcceptanceEnvelope {
    payload: AcceptancePayload,
    signature: String,
}

pub struct AcceptanceReceipt {
    envelope: AcceptanceEnvelope,
    source: Option<OpenReceiptFile>,
}

impl AcceptanceReceipt {
    pub fn open(path: &Path) -> Result<Self> {
        let source = open_bounded(path)?;
        let mut receipt = Self::from_bytes(source.bytes())?;
        receipt.source = Some(source);
        Ok(receipt)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes_sha256(&PINNED_ACCEPTANCE_PUBLIC_KEY) == PINNED_ACCEPTANCE_KEY_FINGERPRINT,
            RECEIPT_AUTHORITY_ERROR
        );
        Self::from_bytes_with_key_impl(bytes, &PINNED_ACCEPTANCE_PUBLIC_KEY)
    }

    fn from_bytes_with_key_impl(bytes: &[u8], public_key: &[u8]) -> Result<Self> {
        reject_duplicate_fields(bytes)?;
        let envelope: AcceptanceEnvelope =
            serde_json::from_slice(bytes).context(RECEIPT_PARSE_ERROR)?;
        ensure!(
            canonical_envelope(&envelope)? == bytes,
            RECEIPT_CANONICAL_ERROR
        );
        validate_payload(&envelope.payload)?;
        crate::crypto::verify_signature_with_key(
            &canonical_payload(&envelope.payload)?,
            &envelope.signature,
            public_key,
        )?;
        Ok(Self {
            envelope,
            source: None,
        })
    }

    #[cfg(feature = "yolo-test-support")]
    pub fn from_bytes_with_key(bytes: &[u8], public_key: &[u8]) -> Result<Self> {
        Self::from_bytes_with_key_impl(bytes, public_key)
    }

    pub fn verify(&self, expectation: &AcceptanceExpectation) -> Result<()> {
        if let Some(source) = &self.source {
            source.verify_unchanged()?;
        }
        expectation.validate()?;
        let record = &self.envelope.payload.record;
        ensure!(
            record.source_revision == expectation.candidate.source_revision
                && record.artifact_sha256 == expectation.candidate.artifact_sha256
                && record.provenance_sha256 == expectation.candidate.provenance_sha256,
            RECEIPT_BINDING_ERROR
        );
        ensure!(
            record.checklist_schema == expectation.checklist_schema,
            RECEIPT_CHECKLIST_ERROR
        );
        ensure!(
            record.providers == expectation.providers,
            RECEIPT_PROVIDER_ERROR
        );
        ensure!(
            record.desktop_os == expectation.desktop_os,
            RECEIPT_PROVIDER_ERROR
        );
        ensure!(
            record.remote_os == expectation.remote_os,
            RECEIPT_PROVIDER_ERROR
        );
        ensure!(
            record.operator_identity == expectation.operator_identity,
            RECEIPT_OPERATOR_ERROR
        );
        ensure!(
            record.completed_at_unix_seconds <= expectation.now_unix_seconds,
            RECEIPT_TIME_ERROR
        );
        ensure!(
            expectation.now_unix_seconds < record.expires_at_unix_seconds,
            RECEIPT_EXPIRED_ERROR
        );
        Ok(())
    }

    pub fn record(&self) -> &AcceptanceRecord {
        &self.envelope.payload.record
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        canonical_envelope(&self.envelope)
    }

    pub fn canonical_digest(&self) -> Result<String> {
        if let Some(source) = &self.source {
            source.verify_unchanged()?;
        }
        Ok(bytes_sha256(&self.canonical_bytes()?))
    }
}

fn validate_payload(payload: &AcceptancePayload) -> Result<()> {
    ensure!(
        payload.schema == ACCEPTANCE_ENVELOPE_SCHEMA,
        RECEIPT_SCHEMA_ERROR
    );
    ensure!(
        payload.authority == ACCEPTANCE_AUTHORITY,
        RECEIPT_AUTHORITY_ERROR
    );
    ensure!(
        payload.algorithm == ACCEPTANCE_ALGORITHM,
        RECEIPT_AUTHORITY_ERROR
    );
    payload.record.validate_shape()
}

fn canonical_payload(payload: &AcceptancePayload) -> Result<Vec<u8>> {
    serde_json::to_vec(payload).context(RECEIPT_PARSE_ERROR)
}

fn canonical_envelope(envelope: &AcceptanceEnvelope) -> Result<Vec<u8>> {
    let mut bytes = serde_json::to_vec(envelope).context(RECEIPT_PARSE_ERROR)?;
    bytes.push(b'\n');
    Ok(bytes)
}
