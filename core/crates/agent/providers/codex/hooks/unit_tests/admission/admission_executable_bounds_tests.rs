use super::{
    CodexAdmissionRecord, CodexExecutableAttestation, CodexExecutableIdentity, CodexExecutableKind,
};
use std::os::unix::fs::FileExt;
#[path = "../../test_support/executable_bounds.rs"]
mod bounds;
#[path = "../../test_support/admission.rs"]
mod support;
use bounds::*;

#[test]
fn provider_accepts_the_packaged_codex_and_larger_images() {
    let fixture = support::Fixture::new();
    for length in [320_248_296, ABOVE_RUNTIME_LIMIT] {
        let file = resize(&fixture.request.real_executable, length);
        CodexExecutableIdentity::inspect(&file.metadata().unwrap(), CodexExecutableKind::Provider)
            .unwrap();
    }
    let empty = resize(&fixture.request.real_executable, 0);
    assert_invalid(CodexExecutableIdentity::inspect(
        &empty.metadata().unwrap(),
        CodexExecutableKind::Provider,
    ));
}

#[test]
fn runtime_size_limit_is_checked_against_real_file_metadata() {
    let fixture = support::Fixture::new();
    let maximum = yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES;
    for length in [0, maximum, ABOVE_RUNTIME_LIMIT] {
        let file = resize(&fixture.request.hook_executable, length);
        assert_eq!(
            CodexExecutableIdentity::inspect(
                &file.metadata().unwrap(),
                CodexExecutableKind::Runtime
            )
            .is_ok(),
            length == maximum
        );
        if length != maximum {
            assert_invalid(CodexExecutableAttestation::capture(
                &fixture.request.hook_executable,
                CodexExecutableKind::Runtime,
            ));
        }
    }
}

#[test]
fn large_provider_hash_includes_the_tail_and_the_runtime_field_enforces_its_own_limit() {
    let fixture = support::Fixture::new();
    let file = resize(&fixture.request.real_executable, ABOVE_RUNTIME_LIMIT);
    let record = fixture.record();
    record.validate_for(&fixture.identity).unwrap();
    let original = record.real_executable();
    original
        .verify_current(CodexExecutableKind::Provider)
        .unwrap();
    assert_invalid(CodexExecutableAttestation::capture(
        original.path(),
        CodexExecutableKind::Runtime,
    ));
    assert_invalid(original.validate_for(original.path(), CodexExecutableKind::Runtime));
    assert_invalid(original.verify_current(CodexExecutableKind::Runtime));
    assert_invalid(
        original.verify_mapped_process(std::process::id(), CodexExecutableKind::Runtime),
    );

    // Field ownership supplies the kind after deserialization. No serialized
    // assertion can turn provider evidence into trusted runtime evidence.
    let mut forged = record.clone();
    forged.request.hook_executable = original.path().to_path_buf();
    forged.hook_executable = original.clone();
    let decoded: CodexAdmissionRecord =
        serde_json::from_slice(&serde_json::to_vec(&forged).unwrap()).unwrap();
    assert_invalid(decoded.validate_for(&fixture.identity));
    assert_invalid(decoded.verify_executables());
    assert_invalid(CodexAdmissionRecord::capture(&decoded.request));
    let mut encoded = serde_json::to_value(original).unwrap();
    assert!(encoded.get(KIND_FIELD).is_none());
    encoded[KIND_FIELD] = RUNTIME_KIND.into();
    assert!(serde_json::from_value::<CodexExecutableAttestation>(encoded).is_err());

    file.write_all_at(&[CHANGED_LAST_BYTE], ABOVE_RUNTIME_LIMIT - BOUNDARY_STEP)
        .unwrap();
    assert!(
        original
            .verify_current(CodexExecutableKind::Provider)
            .is_err()
    );
    let replacement =
        CodexExecutableAttestation::capture(original.path(), CodexExecutableKind::Provider)
            .unwrap();
    assert_ne!(replacement.content_digest(), original.content_digest());
}
