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
fn runtime_and_provider_limits_are_checked_against_real_file_metadata() {
    let fixture = support::Fixture::new();
    for (kind, maximum) in [
        (
            CodexExecutableKind::Provider,
            crate::codex_hooks::MAX_PROVIDER_EXECUTABLE_BYTES,
        ),
        (
            CodexExecutableKind::Runtime,
            yolo_build_image::MAX_EXECUTABLE_IMAGE_BYTES,
        ),
    ] {
        for length in [u64::default(), maximum, maximum + BOUNDARY_STEP] {
            let file = resize(&fixture.request.hook_executable, length);
            assert_eq!(
                CodexExecutableIdentity::inspect(&file.metadata().unwrap(), kind).is_ok(),
                length == maximum
            );
            if length != maximum {
                assert_invalid(CodexExecutableAttestation::capture(
                    &fixture.request.hook_executable,
                    kind,
                ));
            }
        }
    }
}

#[test]
fn a_runtime_sized_proof_cannot_be_reused_as_provider_authority_or_hide_tail_changes() {
    let fixture = support::Fixture::new();
    let file = resize(&fixture.request.hook_executable, RUNTIME_SIZED_EXECUTABLE);
    let record = fixture.record();
    record.validate_for(&fixture.identity).unwrap();
    let original = record.hook_executable();
    assert_invalid(CodexExecutableAttestation::capture(
        original.path(),
        CodexExecutableKind::Provider,
    ));
    assert_invalid(original.validate_for(original.path(), CodexExecutableKind::Provider));
    assert_invalid(original.verify_current(CodexExecutableKind::Provider));
    assert_invalid(
        original.verify_mapped_process(std::process::id(), CodexExecutableKind::Provider),
    );

    // Field ownership supplies the kind after deserialization. No serialized
    // assertion can turn provider evidence into trusted runtime evidence.
    let mut forged = record.clone();
    forged.request.real_executable = original.path().to_path_buf();
    forged.real_executable = original.clone();
    let decoded: CodexAdmissionRecord =
        serde_json::from_slice(&serde_json::to_vec(&forged).unwrap()).unwrap();
    assert_invalid(decoded.validate_for(&fixture.identity));
    assert_invalid(decoded.verify_executables());
    assert_invalid(CodexAdmissionRecord::capture(&decoded.request));
    let mut encoded = serde_json::to_value(original).unwrap();
    assert!(encoded.get(KIND_FIELD).is_none());
    encoded[KIND_FIELD] = RUNTIME_KIND.into();
    assert!(serde_json::from_value::<CodexExecutableAttestation>(encoded).is_err());

    file.write_all_at(
        &[CHANGED_LAST_BYTE],
        RUNTIME_SIZED_EXECUTABLE - BOUNDARY_STEP,
    )
    .unwrap();
    assert!(
        original
            .verify_current(CodexExecutableKind::Runtime)
            .is_err()
    );
    let replacement =
        CodexExecutableAttestation::capture(original.path(), CodexExecutableKind::Runtime).unwrap();
    assert_ne!(replacement.content_digest(), original.content_digest());
}
