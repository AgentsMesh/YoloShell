use super::acceptance_receipt_support::{
    EXPIRY_SECONDS, NOW_SECONDS, bytes, expectation, open, parse, record, replace_once,
};
use crate::constants::READ_ONLY_FILE_MODE;
use crate::constants::{
    ACCEPTANCE_RECORD_SCHEMA, PINNED_ACCEPTANCE_KEY_FINGERPRINT, PINNED_ACCEPTANCE_PUBLIC_KEY,
};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::tempdir;
use yolo_release_foundation::digest::bytes_sha256;

#[test]
fn signed_receipt_verifies_against_exact_expectation() {
    let receipt = parse(&bytes()).unwrap();
    receipt.verify(&expectation()).unwrap();
}

#[test]
fn pinned_authority_fingerprint_matches_public_key() {
    assert_eq!(
        bytes_sha256(&PINNED_ACCEPTANCE_PUBLIC_KEY),
        PINNED_ACCEPTANCE_KEY_FINGERPRINT
    );
}

#[test]
fn production_authority_rejects_test_authority_receipt() {
    assert!(crate::AcceptanceReceipt::from_bytes(&bytes()).is_err());
}

#[test]
fn canonical_bytes_round_trip_without_mutation() {
    let original = bytes();
    let receipt = parse(&original).unwrap();
    assert_eq!(receipt.canonical_bytes().unwrap(), original);
}

#[test]
fn bounded_file_open_accepts_a_canonical_receipt() {
    let directory = tempdir().unwrap();
    let path = directory.path().join("receipt.json");
    fs::write(&path, bytes()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
    let receipt = open(&path).unwrap();
    receipt.verify(&expectation()).unwrap();
}

#[test]
fn candidate_artifact_digest_rebinding_is_rejected() {
    let receipt = parse(&bytes()).unwrap();
    let mut expected = expectation();
    expected.candidate.artifact_sha256 = expected.candidate.provenance_sha256.clone();
    assert!(receipt.verify(&expected).is_err());
}

#[test]
fn candidate_source_revision_rebinding_is_rejected() {
    let receipt = parse(&bytes()).unwrap();
    let mut expected = expectation();
    expected.candidate.source_revision = "fedcba9876543210fedcba9876543210fedcba98".to_owned();
    assert!(receipt.verify(&expected).is_err());
}

#[test]
fn candidate_provenance_digest_rebinding_is_rejected() {
    let receipt = parse(&bytes()).unwrap();
    let mut expected = expectation();
    expected.candidate.provenance_sha256 = expected.candidate.artifact_sha256.clone();
    assert!(receipt.verify(&expected).is_err());
}

#[test]
fn expiry_boundary_is_rejected() {
    let mut expected = expectation();
    expected.now_unix_seconds = EXPIRY_SECONDS;
    let receipt = parse(&bytes()).unwrap();
    assert!(receipt.verify(&expected).is_err());
}

#[test]
fn future_completion_is_rejected() {
    let mut expected = expectation();
    expected.now_unix_seconds = NOW_SECONDS - 120;
    let receipt = parse(&bytes()).unwrap();
    assert!(receipt.verify(&expected).is_err());
}

#[test]
fn schema_mutation_is_rejected_before_signature_acceptance() {
    let original = format!(r#""schema":{ACCEPTANCE_RECORD_SCHEMA}"#);
    let changed = format!(r#""schema":{}"#, ACCEPTANCE_RECORD_SCHEMA + 1);
    let mutated = replace_once(&bytes(), &original, &changed);
    assert!(parse(&mutated).is_err());
}

#[test]
fn receipt_replacement_with_a_different_candidate_is_rejected() {
    let original = bytes();
    let rebound = replace_once(
        &original,
        &record().source_revision,
        "fedcba9876543210fedcba9876543210fedcba98",
    );
    assert!(parse(&rebound).is_err());
}
