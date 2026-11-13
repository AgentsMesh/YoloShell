use super::developer_release_acceptance_support::{
    GOLDEN_RECEIPT_EXPIRED_NOW, GOLDEN_RECEIPT_NOW, replace_durable_acceptance_digest,
    write_acceptance_receipt,
};
use super::developer_release_fixture::{DeveloperReleaseFixture, FakeDeveloperRunner};
use super::developer_release_replacement_runner::{ReplacementMoment, ReplacingDeveloperRunner};
use crate::developer_release::{
    DeveloperReleaseCheckpoint, DeveloperReleaseHooks, distribute_public_with_at,
    distribute_public_with_hooks_at, distribute_with_hooks,
};
use anyhow::{Result, bail};
use std::fs;
use std::sync::atomic::Ordering;

const PREPARED_CHECKPOINT_FAILURE: &str = "injected acceptance prepared checkpoint failure";
const REPLACED_RECEIPT_NAME: &str = "superseded-acceptance.json";
const CHANGED_RECEIPT_BYTES: &[u8] = b"{}\n";

#[test]
fn receipt_path_replacement_before_signing_fails_before_codesign() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    let runner = ReplacingDeveloperRunner::receipt(receipt.clone(), ReplacementMoment::BeforeSign);
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_NOW).is_err());
    assert_eq!(runner.signings(), 0);
    assert_eq!(runner.notarizations(), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn candidate_path_replacement_after_authorization_fails_before_codesign() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    let runner =
        ReplacingDeveloperRunner::candidate_before_sign(fixture.delivery.candidate.clone());
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_NOW).is_err());
    assert_eq!(runner.signings(), 0);
    assert_eq!(runner.notarizations(), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn receipt_path_replacement_before_notarization_fails_before_submission() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    let runner =
        ReplacingDeveloperRunner::receipt(receipt.clone(), ReplacementMoment::BeforeNotarize);
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_NOW).is_err());
    assert_eq!(runner.signings(), 1);
    assert_eq!(runner.notarizations(), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn prepared_public_recovery_accepts_same_receipt_after_expiry() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    prepare_public_distribution(&fixture, &receipt);
    let runner = FakeDeveloperRunner::passing();
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert_eq!(
        distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_EXPIRED_NOW).unwrap(),
        fixture.delivery.expected
    );
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 0);
    assert!(fixture.delivery.delivery.is_dir());
}

#[test]
fn prepared_public_recovery_rejects_a_missing_receipt() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    prepare_public_distribution(&fixture, &receipt);
    fs::remove_file(&receipt).unwrap();
    let runner = FakeDeveloperRunner::passing();
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_EXPIRED_NOW).is_err());
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn prepared_public_recovery_rejects_a_changed_receipt() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    prepare_public_distribution(&fixture, &receipt);
    fs::rename(&receipt, receipt.with_file_name(REPLACED_RECEIPT_NAME)).unwrap();
    fs::write(&receipt, CHANGED_RECEIPT_BYTES).unwrap();
    let runner = FakeDeveloperRunner::passing();
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_EXPIRED_NOW).is_err());
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn prepared_public_recovery_rejects_a_durable_digest_mismatch() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    prepare_public_distribution(&fixture, &receipt);
    replace_durable_acceptance_digest(&fixture, &receipt);
    let runner = FakeDeveloperRunner::passing();
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_EXPIRED_NOW).is_err());
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn receipt_replacement_during_prepared_recovery_prevents_publication() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    prepare_public_distribution(&fixture, &receipt);
    let runner = ReplacingDeveloperRunner::receipt(
        receipt.clone(),
        ReplacementMoment::DuringRecoveryVerification,
    );
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_EXPIRED_NOW).is_err());
    assert_eq!(runner.notarizations(), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn public_recovery_rejects_a_durable_binding_without_acceptance_digest() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    let runner = FakeDeveloperRunner::passing();
    assert!(distribute_with_hooks(fixture.request(), &runner, &FailBeforeRename).is_err());
    let recovery = FakeDeveloperRunner::passing();
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    assert!(distribute_public_with_at(request, &recovery, GOLDEN_RECEIPT_NOW).is_err());
    assert_eq!(recovery.notarizations.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
}

fn prepare_public_distribution(fixture: &DeveloperReleaseFixture, receipt: &std::path::Path) {
    let runner = FakeDeveloperRunner::passing();
    let mut request = fixture.request();
    request.acceptance_receipt = Some(receipt);
    assert!(
        distribute_public_with_hooks_at(request, &runner, &FailBeforeRename, GOLDEN_RECEIPT_NOW,)
            .is_err()
    );
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 1);
    assert!(!fixture.delivery.delivery.exists());
}

struct FailBeforeRename;

impl DeveloperReleaseHooks for FailBeforeRename {
    fn checkpoint(&self, checkpoint: DeveloperReleaseCheckpoint) -> Result<()> {
        assert_eq!(checkpoint, DeveloperReleaseCheckpoint::PreparedBeforeRename);
        bail!(PREPARED_CHECKPOINT_FAILURE)
    }
}
