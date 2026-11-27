use super::support::error_text;
use crate::GateSnapshot;
use crate::coverage_report_io::{CoverageDirectory, create_file, reopen_file};
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::symlink;
use yolo_release_contracts::{
    RUST_COVERAGE_CUSTODY_DIRECTORY_NAME, RUST_COVERAGE_CUSTODY_FILE_NAME,
    RUST_COVERAGE_DIRECTORY_CREATE_ERROR, RUST_COVERAGE_FILE_CREATE_ERROR,
    RUST_COVERAGE_FILE_OPEN_ERROR,
};

const COVERAGE_BYTES: &[u8] = b"retained coverage evidence\n";
const FOREIGN_BYTES: &[u8] = b"unrelated replacement\n";
const MOVED_DIRECTORY: &str = "coverage-custody-retained";
const FOREIGN_FILE: &str = "unrelated-report";

#[test]
fn coverage_custody_is_exclusive_and_does_not_overwrite_existing_evidence() {
    let snapshot = GateSnapshot::empty().unwrap();
    let custody = CoverageDirectory::create(snapshot.temporary()).unwrap();
    let mut original = create_file(custody.handle()).unwrap();
    original.write_all(COVERAGE_BYTES).unwrap();
    assert!(error_text(create_file(custody.handle())).contains(RUST_COVERAGE_FILE_CREATE_ERROR));
    assert!(
        error_text(CoverageDirectory::create(snapshot.temporary()))
            .contains(RUST_COVERAGE_DIRECTORY_CREATE_ERROR)
    );
    let mut bytes = Vec::new();
    reopen_file(custody.handle())
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, COVERAGE_BYTES);
}

#[test]
fn coverage_reopen_rejects_missing_names_and_symlink_substitution() {
    let snapshot = GateSnapshot::empty().unwrap();
    let custody = CoverageDirectory::create(snapshot.temporary()).unwrap();
    assert!(error_text(reopen_file(custody.handle())).contains(RUST_COVERAGE_FILE_OPEN_ERROR));
    let foreign = snapshot.temporary().display_path().join(FOREIGN_FILE);
    fs::write(&foreign, FOREIGN_BYTES).unwrap();
    symlink(&foreign, custody.path()).unwrap();
    assert!(error_text(reopen_file(custody.handle())).contains(RUST_COVERAGE_FILE_OPEN_ERROR));
    assert!(error_text(create_file(custody.handle())).contains(RUST_COVERAGE_FILE_CREATE_ERROR));
    drop(custody);
    assert_eq!(fs::read(foreign).unwrap(), FOREIGN_BYTES);
}

#[test]
fn coverage_cleanup_preserves_a_rebound_directory_and_its_foreign_report() {
    let snapshot = GateSnapshot::empty().unwrap();
    let custody = CoverageDirectory::create(snapshot.temporary()).unwrap();
    create_file(custody.handle())
        .unwrap()
        .write_all(COVERAGE_BYTES)
        .unwrap();
    let original = snapshot
        .temporary()
        .display_path()
        .join(RUST_COVERAGE_CUSTODY_DIRECTORY_NAME);
    let moved = snapshot.temporary().display_path().join(MOVED_DIRECTORY);
    fs::rename(&original, &moved).unwrap();
    fs::create_dir(&original).unwrap();
    let replacement = original.join(RUST_COVERAGE_CUSTODY_FILE_NAME);
    fs::write(&replacement, FOREIGN_BYTES).unwrap();
    drop(custody);
    assert_eq!(fs::read(replacement).unwrap(), FOREIGN_BYTES);
    assert!(!moved.join(RUST_COVERAGE_CUSTODY_FILE_NAME).exists());
    assert!(moved.is_dir());
}

#[test]
fn preexisting_custody_directory_symlink_is_not_followed_or_removed() {
    let snapshot = GateSnapshot::empty().unwrap();
    let foreign = snapshot.temporary().display_path().join(MOVED_DIRECTORY);
    fs::create_dir(&foreign).unwrap();
    fs::write(foreign.join(FOREIGN_FILE), FOREIGN_BYTES).unwrap();
    let custody = snapshot
        .temporary()
        .display_path()
        .join(RUST_COVERAGE_CUSTODY_DIRECTORY_NAME);
    symlink(&foreign, &custody).unwrap();
    assert!(
        error_text(CoverageDirectory::create(snapshot.temporary()))
            .contains(RUST_COVERAGE_DIRECTORY_CREATE_ERROR)
    );
    assert!(fs::symlink_metadata(custody).unwrap().is_symlink());
    assert_eq!(fs::read(foreign.join(FOREIGN_FILE)).unwrap(), FOREIGN_BYTES);
}
