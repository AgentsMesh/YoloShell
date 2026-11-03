use super::support::error_text;
use crate::GateSnapshot;
use std::fs;
use std::os::unix::fs::symlink;
use yolo_release_contracts::LINUX_RUNTIME_MANIFEST_NAME;

const SOURCE_DIRECTORY: &str = "nested";
const SOURCE_NAME: &str = "nested/input.rs";
const SOURCE_BYTES: &[u8] = b"pub const VALUE: bool = true;\n";
const MISSING_SOURCE: &str = "nested/missing.rs";
const SYMLINK_SOURCE: &str = "source-link.rs";

#[test]
fn source_access_rejects_empty_traversing_missing_and_nonfile_inputs() {
    let snapshot = GateSnapshot::fixture_with_source(&[(SOURCE_NAME, SOURCE_BYTES)]).unwrap();
    assert!(error_text(snapshot.open_source_file("")).contains("must not be empty"));
    for relative in [
        "../input.rs",
        "/input.rs",
        "nested/../input.rs",
        "./input.rs",
    ] {
        assert!(
            error_text(snapshot.open_source_file(relative)).contains("relative and normalized")
        );
    }
    assert!(
        error_text(snapshot.open_source_file(MISSING_SOURCE)).contains("open fixed source file")
    );
    assert!(error_text(snapshot.open_source_file(SOURCE_DIRECTORY)).contains("must be a file"));
    let source = snapshot.open_source_file(SOURCE_NAME).unwrap();
    assert_eq!(source.metadata().unwrap().len(), SOURCE_BYTES.len() as u64);
}

#[test]
fn fixed_source_access_cannot_follow_a_symbolic_link() {
    let snapshot = GateSnapshot::empty().unwrap();
    let target = snapshot.source().display_path().join(SOURCE_NAME);
    fs::create_dir_all(target.parent().unwrap()).unwrap();
    fs::write(&target, SOURCE_BYTES).unwrap();
    symlink(
        &target,
        snapshot.source().display_path().join(SYMLINK_SOURCE),
    )
    .unwrap();
    assert!(
        error_text(snapshot.open_source_file(SYMLINK_SOURCE)).contains("open fixed source file")
    );
    assert_eq!(fs::read(target).unwrap(), SOURCE_BYTES);
}

#[test]
fn linux_manifest_requires_a_real_regular_file() {
    let snapshot = GateSnapshot::empty().unwrap();
    let path = snapshot
        .linux_artifacts()
        .display_path()
        .join(LINUX_RUNTIME_MANIFEST_NAME);
    assert!(error_text(snapshot.open_linux_manifest()).contains("open Linux runtime manifest"));
    fs::create_dir(&path).unwrap();
    assert!(error_text(snapshot.open_linux_manifest()).contains("must be a file"));
    fs::remove_dir(&path).unwrap();
    fs::write(&path, SOURCE_BYTES).unwrap();
    assert_eq!(
        snapshot
            .open_linux_manifest()
            .unwrap()
            .metadata()
            .unwrap()
            .len(),
        SOURCE_BYTES.len() as u64
    );
}
