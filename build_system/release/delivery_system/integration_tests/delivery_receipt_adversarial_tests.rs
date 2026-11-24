use super::delivery_fixture::{
    CONTENTS_DIRECTORY_ENTRY, DeliveryFixture, ExtractionKind, FakeRunner, PAYLOAD_ENTRY,
    result_error,
};
use super::delivery_recovery_support::receipt_path;
use crate::delivery::{DeliveryHooks, DeliveryRequest, deliver_with};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_release_archive_audit::constants::ROOT_DIRECTORY_ENTRY;
use yolo_release_archive_audit_test_support::{FixtureEntry, build};
use yolo_release_contracts::DELIVERY_RECOVERY_CONFLICT_ERROR;
use yolo_release_contracts::{PRIVATE_FILE_MODE, READ_ONLY_FILE_MODE};

const CORRUPT_RECEIPT_BYTES: &[u8] = b"corrupt-receipt";
const ALTERNATE_DESTINATION: &str = "alternate-delivery";
const OTHER_ARCHIVE_ENTRY: &[u8] = b"Yolo.app/Contents/other";

struct NoopHooks;

impl DeliveryHooks for NoopHooks {}

#[test]
fn malformed_receipt_is_rejected_without_touching_bound_output() {
    let fixture = DeliveryFixture::new();
    deliver_with(
        fixture.request(),
        &FakeRunner::new(ExtractionKind::Exact, false),
        &NoopHooks,
    )
    .unwrap();
    let receipt = receipt_path(&fixture.delivery);
    set_mode(&receipt, PRIVATE_FILE_MODE);
    fs::write(&receipt, CORRUPT_RECEIPT_BYTES).unwrap();
    set_mode(&receipt, READ_ONLY_FILE_MODE);

    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(fixture.delivery.is_dir());
    assert_eq!(runner.extraction_calls(), 0);
    assert_eq!(runner.signature_calls(), 0);
}

#[test]
fn receipt_rejects_a_different_verified_candidate() {
    let fixture = DeliveryFixture::new();
    deliver_with(
        fixture.request(),
        &FakeRunner::new(ExtractionKind::Exact, false),
        &NoopHooks,
    )
    .unwrap();
    let other = different_candidate();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(
        DeliveryRequest {
            candidate_directory: &other.candidate,
            output_directory: &fixture.delivery,
        },
        &runner,
        &NoopHooks,
    ));

    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(fixture.delivery.is_dir());
    assert_eq!(runner.extraction_calls(), 0);
    assert_eq!(runner.signature_calls(), 0);
}

#[test]
fn receipt_cannot_be_moved_to_a_different_parent() {
    let source = DeliveryFixture::new();
    deliver_with(
        source.request(),
        &FakeRunner::new(ExtractionKind::Exact, false),
        &NoopHooks,
    )
    .unwrap();
    let target = DeliveryFixture::new();
    fs::copy(
        receipt_path(&source.delivery),
        receipt_path(&target.delivery),
    )
    .unwrap();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(target.request(), &runner, &NoopHooks));

    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(!target.delivery.exists());
    assert_eq!(runner.extraction_calls(), 0);
    assert_eq!(runner.signature_calls(), 0);
}

#[test]
fn receipt_cannot_be_rebound_to_a_different_destination_component() {
    let fixture = DeliveryFixture::new();
    deliver_with(
        fixture.request(),
        &FakeRunner::new(ExtractionKind::Exact, false),
        &NoopHooks,
    )
    .unwrap();
    let alternate = fixture
        .delivery
        .parent()
        .unwrap()
        .join(ALTERNATE_DESTINATION);
    fs::copy(receipt_path(&fixture.delivery), receipt_path(&alternate)).unwrap();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(
        DeliveryRequest {
            candidate_directory: &fixture.candidate,
            output_directory: &alternate,
        },
        &runner,
        &NoopHooks,
    ));

    assert!(error.contains(DELIVERY_RECOVERY_CONFLICT_ERROR), "{error}");
    assert!(!alternate.exists());
    assert!(fixture.delivery.is_dir());
    assert_eq!(runner.extraction_calls(), 0);
    assert_eq!(runner.signature_calls(), 0);
}

fn different_candidate() -> DeliveryFixture {
    DeliveryFixture::from_archive(
        build(&[
            FixtureEntry::directory(ROOT_DIRECTORY_ENTRY.as_bytes()),
            FixtureEntry::directory(CONTENTS_DIRECTORY_ENTRY),
            FixtureEntry::regular(PAYLOAD_ENTRY),
            FixtureEntry::regular(OTHER_ARCHIVE_ENTRY),
        ])
        .bytes,
    )
}

fn set_mode(path: &std::path::Path, mode: u32) {
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
}
