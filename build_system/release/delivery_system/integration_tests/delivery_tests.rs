use super::delivery_fixture::{
    COMPLETE_DELIVERY_SIGNATURE_CALLS, DeliveryFixture, ExtractionKind, FakeRunner,
    INITIAL_EXTRACTION_CALLS, INJECTED_SIGNATURE_FAILURE, PAYLOAD_PATH,
    PREPUBLICATION_SIGNATURE_CALLS, result_error,
};
use crate::delivery::{DeliveryHooks, deliver_with};
use crate::delivery_command::{extraction_arguments, signature_arguments};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::{
    APP_NAME, CODESIGN_ARGUMENTS, DELIVERY_EXISTING_OUTPUT_ERROR, DELIVERY_OUTPUT_FILE_NAMES,
    DITTO_ARGUMENTS, EXTRACTION_DIRECTORY_NAME,
};
use yolo_release_contracts::{
    ARCHIVE_NAME, CHECKSUM_NAME, PERMISSION_BITS_MASK, READ_ONLY_FILE_MODE,
};
use yolo_release_verification::validation::checksum_contents;

struct NoopHooks;

impl DeliveryHooks for NoopHooks {}

#[test]
fn exact_candidate_delivers_exact_independent_outputs() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let actual = deliver_with(fixture.request(), &runner, &NoopHooks).unwrap();
    assert_eq!(actual, fixture.expected);
    let mut names = fs::read_dir(&fixture.delivery)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    names.sort();
    let mut expected = DELIVERY_OUTPUT_FILE_NAMES
        .iter()
        .map(|name| (*name).to_owned())
        .collect::<Vec<_>>();
    expected.sort();
    assert_eq!(names, expected);
    assert_eq!(
        fs::read(fixture.delivery.join(ARCHIVE_NAME)).unwrap(),
        fixture.archive
    );
    assert_eq!(
        fs::read(fixture.delivery.join(CHECKSUM_NAME)).unwrap(),
        checksum_contents(&fixture.expected.artifact.sha256).as_bytes()
    );
    assert!(fixture.delivery.join(APP_NAME).is_dir());
    assert!(fixture.delivery.join(PAYLOAD_PATH).is_file());
    for name in [ARCHIVE_NAME, CHECKSUM_NAME] {
        assert_eq!(
            fs::metadata(fixture.delivery.join(name))
                .unwrap()
                .permissions()
                .mode()
                & PERMISSION_BITS_MASK,
            READ_ONLY_FILE_MODE
        );
    }
    assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
    assert_eq!(runner.signature_calls(), COMPLETE_DELIVERY_SIGNATURE_CALLS);
}

#[test]
fn existing_destination_is_preserved_without_running_delivery_commands() {
    let fixture = DeliveryFixture::new();
    fs::create_dir(&fixture.delivery).unwrap();
    let marker = fixture.delivery.join("preserved");
    fs::write(&marker, b"existing").unwrap();
    let runner = FakeRunner::new(ExtractionKind::Exact, false);
    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains(DELIVERY_EXISTING_OUTPUT_ERROR), "{error}");
    assert_eq!(fs::read(marker).unwrap(), b"existing");
    assert_eq!(runner.extraction_calls(), 0);
    assert_eq!(runner.signature_calls(), 0);
}

#[test]
fn signature_failure_is_audited_and_publishes_nothing() {
    let fixture = DeliveryFixture::new();
    let runner = FakeRunner::new(ExtractionKind::Exact, true);
    let error = result_error(deliver_with(fixture.request(), &runner, &NoopHooks));
    assert!(error.contains(INJECTED_SIGNATURE_FAILURE), "{error}");
    assert_eq!(runner.extraction_calls(), INITIAL_EXTRACTION_CALLS);
    assert_eq!(runner.signature_calls(), PREPUBLICATION_SIGNATURE_CALLS);
    assert!(!fixture.delivery.exists());
    fixture.assert_no_stage();
}

#[test]
fn production_delivery_commands_have_only_fixed_arguments() {
    let expected_extract = DITTO_ARGUMENTS
        .iter()
        .copied()
        .chain([ARCHIVE_NAME, EXTRACTION_DIRECTORY_NAME])
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let expected_signature = CODESIGN_ARGUMENTS
        .iter()
        .copied()
        .chain([APP_NAME])
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert_eq!(extraction_arguments(), expected_extract);
    assert_eq!(signature_arguments(), expected_signature);
}
