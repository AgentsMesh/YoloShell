use super::support::{
    ARCHIVE_BYTES, BAZEL_OUTPUT_ORIGINAL_NAME, BAZEL_OUTPUT_REVISION_A, BAZEL_OUTPUT_REVISION_B,
    OTHER_ARCHIVE_BYTES, OutputFixture, create_archive, error_text,
};
use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::{
    BAZEL_OUTPUT_CONTAINMENT_ERROR, BAZEL_OUTPUT_FILE_CHANGED_ERROR,
    BAZEL_OUTPUT_LINK_CHANGED_ERROR, PACKAGE_ARTIFACT_RELATIVE_PATH, READ_ONLY_FILE_MODE,
};

#[test]
fn stable_retained_output_is_verified() {
    let fixture = OutputFixture::new();
    let output = fixture.create_output(BAZEL_OUTPUT_REVISION_A, ARCHIVE_BYTES);
    fixture.point_link(&output);
    let binding = fixture.binding().unwrap();
    let mut bytes = Vec::new();
    binding.file().read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, ARCHIVE_BYTES);
    binding.verify().unwrap();
}

#[test]
fn output_link_swap_cannot_redirect_pinned_revision() {
    let fixture = OutputFixture::new();
    let first = fixture.create_output(BAZEL_OUTPUT_REVISION_A, ARCHIVE_BYTES);
    let second = fixture.create_output(BAZEL_OUTPUT_REVISION_B, OTHER_ARCHIVE_BYTES);
    fixture.point_link(&first);
    let binding = fixture.binding().unwrap();
    fixture.point_link(&second);
    let mut bytes = Vec::new();
    binding.file().read_to_end(&mut bytes).unwrap();
    assert_eq!(bytes, ARCHIVE_BYTES);
    assert!(error_text(binding.verify()).contains(BAZEL_OUTPUT_LINK_CHANGED_ERROR));
}

#[test]
fn output_link_cannot_escape_retained_root() {
    let fixture = OutputFixture::new();
    let external = tempfile::tempdir().unwrap();
    let target = external.path().join(BAZEL_OUTPUT_REVISION_B);
    create_archive(&target, OTHER_ARCHIVE_BYTES);
    fixture.point_link(&target);
    assert!(error_text(fixture.binding()).contains(BAZEL_OUTPUT_CONTAINMENT_ERROR));
}

#[test]
fn terminal_output_file_replacement_is_rejected() {
    let fixture = OutputFixture::new();
    let output = fixture.create_output(BAZEL_OUTPUT_REVISION_A, ARCHIVE_BYTES);
    fixture.point_link(&output);
    let binding = fixture.binding().unwrap();
    let archive = output.join(PACKAGE_ARTIFACT_RELATIVE_PATH);
    fs::rename(&archive, archive.with_file_name(BAZEL_OUTPUT_ORIGINAL_NAME)).unwrap();
    fs::write(&archive, OTHER_ARCHIVE_BYTES).unwrap();
    assert!(error_text(binding.verify()).contains(BAZEL_OUTPUT_FILE_CHANGED_ERROR));
}

#[test]
fn terminal_output_file_metadata_change_is_rejected() {
    let fixture = OutputFixture::new();
    let output = fixture.create_output(BAZEL_OUTPUT_REVISION_A, ARCHIVE_BYTES);
    fixture.point_link(&output);
    let binding = fixture.binding().unwrap();
    let archive = output.join(PACKAGE_ARTIFACT_RELATIVE_PATH);
    fs::set_permissions(&archive, fs::Permissions::from_mode(READ_ONLY_FILE_MODE)).unwrap();
    assert!(error_text(binding.verify()).contains(BAZEL_OUTPUT_FILE_CHANGED_ERROR));
}
