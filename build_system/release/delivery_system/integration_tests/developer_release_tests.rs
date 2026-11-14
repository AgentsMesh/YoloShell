use super::developer_release_acceptance_support::{GOLDEN_RECEIPT_NOW, write_acceptance_receipt};
use super::developer_release_fixture::{
    DeveloperReleaseFixture, FakeDeveloperRunner, INJECTED_ASSESSMENT_FAILURE,
    INJECTED_NOTARY_FAILURE, POSTPUBLICATION_ASSESSMENT_CALL,
};
use crate::developer_release::{
    DeveloperReleaseCheckpoint, DeveloperReleaseHooks, distribute_public_with,
    distribute_public_with_at, distribute_with, distribute_with_hooks,
};
use crate::developer_release_constants::PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR;
use anyhow::{Result, bail};
use std::fs;
use std::sync::atomic::Ordering;
use yolo_release_contracts::{APP_NAME, DELIVERY_OUTPUT_FILE_NAMES};
use yolo_release_contracts::{ARCHIVE_NAME, CHECKSUM_NAME};
use yolo_release_foundation::digest::opened_file_sha256;
use yolo_release_verification::validation::checksum_contents;

#[test]
fn unaccepted_candidate_never_reaches_signing_or_notarization() {
    let fixture = DeveloperReleaseFixture::new();
    let runner = FakeDeveloperRunner::passing();
    let error = super::support::error_text(distribute_public_with(fixture.request(), &runner));
    assert!(
        error.contains(PUBLIC_DISTRIBUTION_ACCEPTANCE_ERROR),
        "{error}"
    );
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 0);
    assert_eq!(runner.signature_verifications.load(Ordering::SeqCst), 0);
    assert_eq!(runner.ticket_validations.load(Ordering::SeqCst), 0);
    assert_eq!(runner.assessments.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
    fixture.delivery.assert_no_stage();
}

#[test]
fn signed_acceptance_receipt_allows_public_distribution() {
    let fixture = DeveloperReleaseFixture::new();
    let receipt = write_acceptance_receipt(&fixture);
    let mut request = fixture.request();
    request.acceptance_receipt = Some(&receipt);
    let runner = FakeDeveloperRunner::passing();
    let provenance = distribute_public_with_at(request, &runner, GOLDEN_RECEIPT_NOW).unwrap();
    assert_eq!(provenance, fixture.delivery.expected);
    assert!(fixture.delivery.delivery.is_dir());
}

#[test]
fn accepted_notarization_publishes_signed_distribution_atomically() {
    let fixture = DeveloperReleaseFixture::new();
    let runner = FakeDeveloperRunner::passing();
    let provenance = distribute_with(fixture.request(), &runner).unwrap();
    assert_eq!(provenance, fixture.delivery.expected);
    let mut names = fs::read_dir(&fixture.delivery.delivery)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    let mut expected = DELIVERY_OUTPUT_FILE_NAMES.to_vec();
    expected.sort();
    assert_eq!(names, expected);
    assert!(fixture.delivery.delivery.join(APP_NAME).is_dir());
    let archive = fs::File::open(fixture.delivery.delivery.join(ARCHIVE_NAME)).unwrap();
    let digest = opened_file_sha256(&archive).unwrap();
    assert_eq!(
        fs::read(fixture.delivery.delivery.join(CHECKSUM_NAME)).unwrap(),
        checksum_contents(&digest).as_bytes()
    );
    assert_eq!(runner.signature_verifications.load(Ordering::SeqCst), 4);
    assert_eq!(runner.ticket_validations.load(Ordering::SeqCst), 3);
    assert_eq!(runner.assessments.load(Ordering::SeqCst), 3);
}

#[test]
fn notary_rejection_never_publishes_or_leaves_a_private_stage() {
    let fixture = DeveloperReleaseFixture::new();
    let runner = FakeDeveloperRunner::failing_notary();
    let error = super::support::error_text(distribute_with(fixture.request(), &runner));
    assert!(error.contains(INJECTED_NOTARY_FAILURE), "{error}");
    assert!(!fixture.delivery.delivery.exists());
    fixture.delivery.assert_no_stage();
}

#[test]
fn invalid_profile_is_rejected_before_output_staging() {
    let fixture = DeveloperReleaseFixture::new();
    let runner = FakeDeveloperRunner::passing();
    let mut request = fixture.request();
    request.notary_profile = " leading-space";
    assert!(distribute_with(request, &runner).is_err());
    assert!(!fixture.delivery.delivery.exists());
    fixture.delivery.assert_no_stage();
}

#[test]
fn prepared_receipt_resumes_without_repeating_notarization() {
    let fixture = DeveloperReleaseFixture::new();
    let first = FakeDeveloperRunner::passing();
    assert!(distribute_with_hooks(fixture.request(), &first, &FailBeforeRename).is_err());
    assert!(!fixture.delivery.delivery.exists());
    let recovery = FakeDeveloperRunner::passing();
    assert_eq!(
        distribute_with(fixture.request(), &recovery).unwrap(),
        fixture.delivery.expected
    );
    assert_eq!(recovery.notarizations.load(Ordering::SeqCst), 0);
    assert!(fixture.delivery.delivery.is_dir());
}

#[test]
fn committed_but_unconfirmed_distribution_recovers_in_place() {
    let fixture = DeveloperReleaseFixture::new();
    let first = FakeDeveloperRunner::failing_assessment(POSTPUBLICATION_ASSESSMENT_CALL);
    let error = super::support::error_text(distribute_with(fixture.request(), &first));
    assert!(error.contains(INJECTED_ASSESSMENT_FAILURE), "{error}");
    assert!(fixture.delivery.delivery.is_dir());
    let recovery = FakeDeveloperRunner::passing();
    assert_eq!(
        distribute_with(fixture.request(), &recovery).unwrap(),
        fixture.delivery.expected
    );
    assert_eq!(recovery.notarizations.load(Ordering::SeqCst), 0);
}

struct FailBeforeRename;

impl DeveloperReleaseHooks for FailBeforeRename {
    fn checkpoint(&self, checkpoint: DeveloperReleaseCheckpoint) -> Result<()> {
        assert_eq!(checkpoint, DeveloperReleaseCheckpoint::PreparedBeforeRename);
        bail!("injected prepared checkpoint failure")
    }
}
