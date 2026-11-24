pub use yolo_release_gate_test_support::support::*;

use crate::GateSnapshot;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

pub fn coverage_output(snapshot: &GateSnapshot, revision: &str, bytes: &[u8]) -> PathBuf {
    let target = snapshot.output().display_path().join(revision);
    let report = target.join(yolo_release_contracts::RUST_COVERAGE_REPORT_RELATIVE_PATH);
    fs::create_dir_all(report.parent().unwrap()).unwrap();
    fs::write(report, bytes).unwrap();
    point_coverage_link(snapshot, &target);
    target
}

pub fn point_coverage_link(snapshot: &GateSnapshot, target: &Path) {
    let link = snapshot
        .links()
        .display_path()
        .join(yolo_release_contracts::BAZEL_OUT_LINK_NAME);
    if link.exists() || link.is_symlink() {
        fs::remove_file(&link).unwrap();
    }
    symlink(target, link).unwrap();
}
