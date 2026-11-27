use super::delivery_recovery_support::receipt_path;
use super::developer_release_fixture::DeveloperReleaseFixture;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use yolo_release_acceptance_receipt::constants::*;
use yolo_release_acceptance_receipt::{
    AcceptanceCheckResult, AcceptancePlatform, AcceptanceProviderVersion, AcceptanceRecord,
    open_with_test_key, signed_with_test_authority, test_public_key,
};
use yolo_release_contracts::{PRIVATE_FILE_MODE, READ_ONLY_FILE_MODE};
use yolo_release_verification::acceptance_receipt_candidate::candidate_binding_for;

pub const GOLDEN_RECEIPT_NOW: u64 = 1_787_634_019;
const GOLDEN_RECEIPT_COMPLETED: u64 = GOLDEN_RECEIPT_NOW;
pub const GOLDEN_RECEIPT_EXPIRED_NOW: u64 =
    GOLDEN_RECEIPT_COMPLETED + ACCEPTANCE_MAX_VALIDITY_SECONDS;
const ACCEPTANCE_RECEIPT_NAME: &str = "acceptance.json";
const TEST_MACOS_VERSION: &str = "14.2";
const TEST_OPERATOR_IDENTITY: &str = "alpha-operator@example.test";

pub fn write_acceptance_receipt(fixture: &DeveloperReleaseFixture) -> PathBuf {
    let candidate = yolo_release_verification::verify::VerifiedDeliveryCandidate::open(
        &fixture.delivery.candidate,
    )
    .unwrap();
    let binding = candidate_binding_for(&candidate).unwrap();
    let platform = AcceptancePlatform::new(
        ACCEPTANCE_PLATFORM_NAME,
        TEST_MACOS_VERSION,
        ACCEPTANCE_PLATFORM_ARCHITECTURE,
    )
    .unwrap();
    let record = AcceptanceRecord {
        schema: ACCEPTANCE_RECORD_SCHEMA,
        product: ACCEPTANCE_PRODUCT.to_owned(),
        source_revision: binding.source_revision,
        artifact_sha256: binding.artifact_sha256,
        provenance_sha256: binding.provenance_sha256,
        checklist_schema: ACCEPTANCE_CHECKLIST_SCHEMA.to_owned(),
        providers: vec![
            AcceptanceProviderVersion::new(CLAUDE_PROVIDER, CLAUDE_ACCEPTANCE_VERSION_OUTPUT)
                .unwrap(),
            AcceptanceProviderVersion::new(CODEX_PROVIDER, CODEX_ACCEPTANCE_VERSION_OUTPUT)
                .unwrap(),
        ],
        desktop_os: platform.clone(),
        remote_os: platform,
        operator_identity: TEST_OPERATOR_IDENTITY.to_owned(),
        completed_at_unix_seconds: GOLDEN_RECEIPT_COMPLETED,
        expires_at_unix_seconds: GOLDEN_RECEIPT_EXPIRED_NOW,
        results: REQUIRED_ACCEPTANCE_CHECKS
            .iter()
            .map(|identifier| AcceptanceCheckResult::passed(*identifier).unwrap())
            .collect(),
    };
    let path = fixture
        .delivery
        .candidate
        .parent()
        .unwrap()
        .join(ACCEPTANCE_RECEIPT_NAME);
    fs::write(&path, signed_with_test_authority(record).unwrap()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
    path
}

pub fn replace_durable_acceptance_digest(
    fixture: &DeveloperReleaseFixture,
    acceptance_path: &std::path::Path,
) {
    let acceptance = open_with_test_key(acceptance_path, test_public_key()).unwrap();
    let digest = acceptance.canonical_digest().unwrap();
    let durable = receipt_path(&fixture.delivery.delivery);
    let before = fs::read_to_string(&durable).unwrap();
    let replacement = "0".repeat(SHA256_HEX_CHARACTERS);
    let after = before.replacen(&digest, &replacement, 1);
    assert_ne!(before, after);
    fs::set_permissions(&durable, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    fs::write(&durable, after).unwrap();
    fs::set_permissions(&durable, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
}
