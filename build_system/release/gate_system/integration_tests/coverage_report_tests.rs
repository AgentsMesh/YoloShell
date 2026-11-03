use super::support::{coverage_output, error_text, point_coverage_link};
use super::test_constants::{
    BAZEL_OUTPUT_REVISION_A, BAZEL_OUTPUT_REVISION_B, OTHER_RUST_COVERAGE_BYTES,
    RUST_COVERAGE_BYTES,
};
use crate::coverage_report::PinnedCoverageReport;
use crate::snapshot::GateSnapshot;
use std::fs;
use std::fs::File;
use std::os::unix::fs::PermissionsExt;
use yolo_release_contracts::{
    BAZEL_OUTPUT_LINK_CHANGED_ERROR, MAX_RUST_COVERAGE_REPORT_BYTES, PRIVATE_FILE_MODE,
    RUST_COVERAGE_CHANGED_ERROR, RUST_COVERAGE_CUSTODY_DIRECTORY_NAME,
    RUST_COVERAGE_REPORT_RELATIVE_PATH, RUST_COVERAGE_SIZE_ERROR,
};
use yolo_release_foundation::digest::bytes_sha256;

#[test]
fn stable_coverage_report_is_copied_into_sealed_custody() {
    let snapshot = GateSnapshot::empty().unwrap();
    coverage_output(&snapshot, BAZEL_OUTPUT_REVISION_A, RUST_COVERAGE_BYTES);
    let custody = snapshot
        .temporary
        .display_path()
        .join(RUST_COVERAGE_CUSTODY_DIRECTORY_NAME);
    {
        let report = PinnedCoverageReport::capture(&snapshot).unwrap();
        assert_eq!(report.sha256(), bytes_sha256(RUST_COVERAGE_BYTES));
        assert!(report.path().starts_with(&custody));
    }
    assert!(!custody.exists());
}

#[test]
fn bazel_out_swap_after_copy_is_rejected() {
    let snapshot = GateSnapshot::empty().unwrap();
    coverage_output(&snapshot, BAZEL_OUTPUT_REVISION_A, RUST_COVERAGE_BYTES);
    let second = snapshot.output.display_path().join(BAZEL_OUTPUT_REVISION_B);
    let report = second.join(yolo_release_contracts::RUST_COVERAGE_REPORT_RELATIVE_PATH);
    fs::create_dir_all(report.parent().unwrap()).unwrap();
    fs::write(report, OTHER_RUST_COVERAGE_BYTES).unwrap();
    let error = error_text(PinnedCoverageReport::capture_with_hook(&snapshot, || {
        point_coverage_link(&snapshot, &second);
        Ok(())
    }));
    assert!(error.contains(BAZEL_OUTPUT_LINK_CHANGED_ERROR), "{error}");
}

#[test]
fn sealed_coverage_metadata_mutation_is_rejected() {
    let snapshot = GateSnapshot::empty().unwrap();
    coverage_output(&snapshot, BAZEL_OUTPUT_REVISION_A, RUST_COVERAGE_BYTES);
    let report = PinnedCoverageReport::capture(&snapshot).unwrap();
    fs::set_permissions(report.path(), fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    let error = error_text(report.ensure_unchanged());
    assert!(error.contains(RUST_COVERAGE_CHANGED_ERROR), "{error}");
}

#[test]
fn oversized_coverage_report_is_rejected_before_copying() {
    let snapshot = GateSnapshot::empty().unwrap();
    let target = snapshot.output.display_path().join(BAZEL_OUTPUT_REVISION_A);
    let report = target.join(RUST_COVERAGE_REPORT_RELATIVE_PATH);
    fs::create_dir_all(report.parent().unwrap()).unwrap();
    File::create(&report)
        .unwrap()
        .set_len(MAX_RUST_COVERAGE_REPORT_BYTES + 1)
        .unwrap();
    point_coverage_link(&snapshot, &target);
    let error = error_text(PinnedCoverageReport::capture(&snapshot));
    assert!(error.contains(RUST_COVERAGE_SIZE_ERROR), "{error}");
}
