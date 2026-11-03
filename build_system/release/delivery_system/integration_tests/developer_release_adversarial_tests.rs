use super::delivery_fixture::{DeliveryFixture, ExtractionKind, FakeRunner};
use super::developer_release_fixture::{
    DeveloperReleaseFixture, FakeDeveloperRunner, POSTPUBLICATION_ASSESSMENT_CALL,
    TEST_NOTARY_PROFILE,
};
use super::developer_release_fixture_archive::{DistributedArchiveKind, distributed_archive};
use crate::delivery::{DeliveryHooks, deliver_with};
use crate::developer_release::{
    DeveloperReleaseCheckpoint, DeveloperReleaseHooks, DeveloperReleaseRequest, distribute_with,
    distribute_with_hooks,
};
use crate::developer_release_constants::DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR;
use anyhow::{Result, bail};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use yolo_release_contracts::DELIVERY_TEMPORARY_PREFIX;
use yolo_release_contracts::{ARCHIVE_NAME, CHECKSUM_NAME, PRIVATE_FILE_MODE, READ_ONLY_FILE_MODE};
use yolo_release_foundation::digest::bytes_sha256;
use yolo_release_verification::validation::checksum_contents;

const INJECTED_PREPARED_FAILURE: &str = "injected prepared checkpoint failure";

#[test]
fn signing_cannot_change_executable_content_outside_its_signature() {
    let fixture = DeveloperReleaseFixture::new();
    let runner = FakeDeveloperRunner::mutating_signing();
    let error = super::support::error_text(distribute_with(fixture.request(), &runner));
    assert!(
        error.contains(DEVELOPER_RELEASE_EXECUTABLE_CHANGED_ERROR),
        "{error}"
    );
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn notarization_submission_cannot_diverge_from_the_candidate_resources() {
    let fixture = DeveloperReleaseFixture::new();
    let runner = FakeDeveloperRunner::mutating_submission();
    assert!(distribute_with(fixture.request(), &runner).is_err());
    assert_eq!(runner.notarizations.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
    fixture.delivery.assert_no_stage();
}

#[test]
fn final_archive_must_match_the_verified_application_and_ticket() {
    for kind in [
        DistributedArchiveKind::MutatedExecutable,
        DistributedArchiveKind::MutatedTicket,
    ] {
        let fixture = DeveloperReleaseFixture::new();
        let runner = FakeDeveloperRunner::divergent_archive(kind);
        assert!(distribute_with(fixture.request(), &runner).is_err());
        assert!(!fixture.delivery.delivery.exists());
        fixture.delivery.assert_no_stage();
    }
}

#[test]
fn prepared_recovery_rejects_archive_rebaseline_without_renotarizing() {
    let fixture = DeveloperReleaseFixture::new();
    let first = FakeDeveloperRunner::passing();
    assert!(distribute_with_hooks(fixture.request(), &first, &FailBeforeRename).is_err());
    replace_distribution(&retained_stage(&fixture));
    let recovery = FakeDeveloperRunner::passing();
    assert!(distribute_with(fixture.request(), &recovery).is_err());
    assert_eq!(recovery.notarizations.load(Ordering::SeqCst), 0);
    assert!(!fixture.delivery.delivery.exists());
}

#[test]
fn committed_recovery_rejects_archive_rebaseline_without_renotarizing() {
    let fixture = DeveloperReleaseFixture::new();
    let first = FakeDeveloperRunner::failing_assessment(POSTPUBLICATION_ASSESSMENT_CALL);
    assert!(distribute_with(fixture.request(), &first).is_err());
    replace_distribution(&fixture.delivery.delivery);
    let recovery = FakeDeveloperRunner::passing();
    assert!(distribute_with(fixture.request(), &recovery).is_err());
    assert_eq!(recovery.notarizations.load(Ordering::SeqCst), 0);
}

#[test]
fn destination_rebinding_during_assessment_fails_final_confirmation() {
    let fixture = DeveloperReleaseFixture::new();
    let runner = FakeDeveloperRunner::rebinding(
        POSTPUBLICATION_ASSESSMENT_CALL,
        fixture.delivery.delivery.clone(),
    );
    assert!(distribute_with(fixture.request(), &runner).is_err());
}

#[test]
fn developer_distribution_rejects_an_integrity_delivery_receipt() {
    let fixture = DeliveryFixture::new();
    let delivery_runner = FakeRunner::new(ExtractionKind::Exact, false);
    deliver_with(fixture.request(), &delivery_runner, &NoopDeliveryHooks).unwrap();
    let developer_runner = FakeDeveloperRunner::passing();
    let request = DeveloperReleaseRequest {
        candidate_directory: &fixture.candidate,
        output_directory: &fixture.delivery,
        notary_profile: TEST_NOTARY_PROFILE,
        acceptance_receipt: None,
    };
    assert!(distribute_with(request, &developer_runner).is_err());
    assert_eq!(developer_runner.notarizations.load(Ordering::SeqCst), 0);
    assert_eq!(
        developer_runner
            .signature_verifications
            .load(Ordering::SeqCst),
        0
    );
}

#[test]
fn integrity_delivery_rejects_a_developer_distribution_receipt() {
    let fixture = DeveloperReleaseFixture::new();
    distribute_with(fixture.request(), &FakeDeveloperRunner::passing()).unwrap();
    let delivery_runner = FakeRunner::new(ExtractionKind::Exact, false);
    assert!(
        deliver_with(
            fixture.delivery.request(),
            &delivery_runner,
            &NoopDeliveryHooks,
        )
        .is_err()
    );
    assert_eq!(delivery_runner.extraction_calls(), 0);
    assert_eq!(delivery_runner.signature_calls(), 0);
}

struct FailBeforeRename;

struct NoopDeliveryHooks;

impl DeliveryHooks for NoopDeliveryHooks {}

impl DeveloperReleaseHooks for FailBeforeRename {
    fn checkpoint(&self, checkpoint: DeveloperReleaseCheckpoint) -> Result<()> {
        assert_eq!(checkpoint, DeveloperReleaseCheckpoint::PreparedBeforeRename);
        bail!(INJECTED_PREPARED_FAILURE)
    }
}

fn retained_stage(fixture: &DeveloperReleaseFixture) -> PathBuf {
    fs::read_dir(fixture.delivery.delivery.parent().unwrap())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name().is_some_and(|name| {
                name.to_string_lossy()
                    .starts_with(DELIVERY_TEMPORARY_PREFIX)
            })
        })
        .unwrap()
}

fn replace_distribution(root: &Path) {
    let archive = distributed_archive(DistributedArchiveKind::MutatedExecutable);
    overwrite(&root.join(ARCHIVE_NAME), &archive);
    let checksum = checksum_contents(&bytes_sha256(&archive));
    overwrite(&root.join(CHECKSUM_NAME), checksum.as_bytes());
}

fn overwrite(path: &Path, bytes: &[u8]) {
    fs::set_permissions(path, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
}
