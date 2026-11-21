use super::*;
use crate::agent::hooks::{Generation, ProviderVersion};
use std::os::unix::fs::PermissionsExt;

#[path = "../../test_support/admission.rs"]
mod support;
use support::*;

const LINK_FILE_NAME: &str = "linked-executable";
const HARD_LINK_FILE_NAME: &str = "hard-linked-executable";
const PARENT_LINK_NAME: &str = "linked-parent";
const EMPTY_FILE_NAME: &str = "empty-executable";
const NON_EXECUTABLE_FILE_NAME: &str = "non-executable";
const UNSAFE_FILE_NAME: &str = "unsafe-executable";
const OVERSIZED_FILE_NAME: &str = "oversized-executable";
const OVERSIZED_OFFSET: u64 = 1;
const FUTURE_VERSION_OFFSET: u16 = 1;
const FUTURE_GENERATION_OFFSET: u64 = 1;
const LEGACY_SCHEMA_VERSION: u16 = 1;
const TEST_ROUTE_REVISION: u64 = 1;

#[test]
fn record_round_trips_and_binds_native_context_and_executables() {
    let fixture = Fixture::new();
    let record = fixture.record();
    record.validate_for(&fixture.identity).unwrap();
    record.verify_executables().unwrap();
    assert_eq!(record.route_revision(), TEST_ROUTE_REVISION);
    assert_eq!(record.request(), &fixture.request);
    assert_eq!(
        record.hook_executable().path(),
        fixture.request.hook_executable
    );
    assert_eq!(
        record.real_executable().path(),
        fixture.request.real_executable
    );
    assert_ne!(
        record.real_executable().identity(),
        record.hook_executable().identity()
    );
    assert_eq!(
        CodexExecutableIdentity::inspect(
            &std::fs::metadata(&fixture.request.hook_executable).unwrap(),
            CodexExecutableKind::Runtime
        )
        .unwrap(),
        record.hook_executable().identity()
    );
    assert_eq!(
        record.hook_executable().content_digest(),
        CodexExecutableAttestation::capture(
            &fixture.request.hook_executable,
            CodexExecutableKind::Runtime
        )
        .unwrap()
        .content_digest()
    );
    let encoded = serde_json::to_vec(&record).unwrap();
    let decoded: CodexAdmissionRecord = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, record);
}

#[test]
fn legacy_schema_and_zero_route_revision_are_rejected() {
    let fixture = Fixture::new();
    let mut legacy = fixture.record();
    legacy.schema_version = LEGACY_SCHEMA_VERSION;
    assert!(legacy.validate_for(&fixture.identity).is_err());
    assert!(
        CodexAdmissionRecord::capture_with_route_revision(&fixture.request, u64::default())
            .is_err()
    );
}

#[test]
fn unknown_fields_are_rejected_at_every_dto_layer() {
    let fixture = Fixture::new();
    let original = serde_json::to_value(fixture.record()).unwrap();
    for path in [
        Vec::<&str>::new(),
        vec!["request"],
        vec!["hook_executable"],
        vec!["hook_executable", "identity"],
    ] {
        let mut changed = original.clone();
        let mut target = &mut changed;
        for component in path {
            target = target.get_mut(component).unwrap();
        }
        target
            .as_object_mut()
            .unwrap()
            .insert(UNKNOWN_FIELD.into(), UNKNOWN_VALUE.into());
        assert!(serde_json::from_value::<CodexAdmissionRecord>(changed).is_err());
    }
}

#[test]
fn unpinned_request_fields_fail_before_attestation() {
    let fixture = Fixture::new();
    let mut future_version = fixture.request.clone();
    future_version.provider_version = ProviderVersion::new(
        future_version.provider_version.major,
        future_version.provider_version.minor + FUTURE_VERSION_OFFSET,
        future_version.provider_version.patch,
    );
    assert!(CodexAdmissionRecord::capture(&future_version).is_err());

    let mut future_generation = fixture.request.clone();
    future_generation.identity.generation =
        Generation(future_generation.identity.generation.0 + FUTURE_GENERATION_OFFSET);
    assert!(CodexAdmissionRecord::capture(&future_generation).is_err());
}

#[test]
fn executable_content_and_object_replacement_revoke_live_verification() {
    let fixture = Fixture::new();
    let original = fixture.record();
    fixture.replace_hook(CHANGED_EXECUTABLE_CONTENT);
    assert!(original.verify_executables().is_err());
    assert_ne!(fixture.record(), original);

    let fixture = Fixture::new();
    let original = fixture.record();
    fixture.replace_hook(EXECUTABLE_CONTENT);
    let replacement = fixture.record();
    assert_ne!(replacement, original);
    assert_ne!(
        replacement.hook_executable().identity(),
        original.hook_executable().identity()
    );
}

#[test]
fn unsafe_executable_shapes_are_rejected() {
    let fixture = Fixture::new();
    let directory = fixture.request.hook_executable.parent().unwrap();
    let link = directory.join(LINK_FILE_NAME);
    std::os::unix::fs::symlink(&fixture.request.hook_executable, &link).unwrap();
    assert_capture_rejected(&fixture, link);

    let parent_link = directory.parent().unwrap().join(PARENT_LINK_NAME);
    std::os::unix::fs::symlink(directory, &parent_link).unwrap();
    assert_capture_rejected(
        &fixture,
        parent_link.join(fixture.request.hook_executable.file_name().unwrap()),
    );

    let empty = executable(&directory.join(EMPTY_FILE_NAME), &[]);
    assert_capture_rejected(&fixture, empty);
    let non_executable = executable(
        &directory.join(NON_EXECUTABLE_FILE_NAME),
        EXECUTABLE_CONTENT,
    );
    std::fs::set_permissions(
        &non_executable,
        std::fs::Permissions::from_mode(NON_EXECUTABLE_MODE),
    )
    .unwrap();
    assert_capture_rejected(&fixture, non_executable);
    let unsafe_file = executable(&directory.join(UNSAFE_FILE_NAME), EXECUTABLE_CONTENT);
    std::fs::set_permissions(
        &unsafe_file,
        std::fs::Permissions::from_mode(UNSAFE_EXECUTABLE_MODE),
    )
    .unwrap();
    assert_capture_rejected(&fixture, unsafe_file);

    let hard_link = directory.join(HARD_LINK_FILE_NAME);
    std::fs::hard_link(&fixture.request.hook_executable, &hard_link).unwrap();
    assert!(CodexAdmissionRecord::capture(&fixture.request).is_err());

    let oversized = executable(&directory.join(OVERSIZED_FILE_NAME), EXECUTABLE_CONTENT);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&oversized)
        .unwrap()
        .set_len(yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES + OVERSIZED_OFFSET)
        .unwrap();
    assert_capture_rejected(&fixture, oversized);
}

fn assert_capture_rejected(fixture: &Fixture, executable: std::path::PathBuf) {
    let mut request = fixture.request.clone();
    request.hook_executable = executable;
    assert!(CodexAdmissionRecord::capture(&request).is_err());
}
