use super::support::ARCHIVE_BYTES;
use super::test_constants::BAZEL_OUTPUT_REVISION_A;
use crate::GateSnapshot;
use std::fs;
use std::os::unix::fs::symlink;
use yolo_release_contracts::{BAZEL_BIN_LINK_NAME, PACKAGE_ARTIFACT_RELATIVE_PATH};
use yolo_release_foundation::digest::bytes_sha256;

#[test]
fn stable_retained_snapshot_artifact_is_pinned() {
    let snapshot = GateSnapshot::empty().unwrap();
    let output = snapshot
        .output()
        .display_path()
        .join(BAZEL_OUTPUT_REVISION_A);
    let archive = output.join(PACKAGE_ARTIFACT_RELATIVE_PATH);
    fs::create_dir_all(archive.parent().unwrap()).unwrap();
    fs::write(archive, ARCHIVE_BYTES).unwrap();
    symlink(
        &output,
        snapshot.links().display_path().join(BAZEL_BIN_LINK_NAME),
    )
    .unwrap();
    let artifact = snapshot.pin_artifact().unwrap();
    assert_eq!(artifact.sha256().unwrap(), bytes_sha256(ARCHIVE_BYTES));
}
