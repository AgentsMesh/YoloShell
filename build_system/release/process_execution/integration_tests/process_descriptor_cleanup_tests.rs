use super::process_supervision_tests::Fixture;
use super::support::error_text;
use super::test_constants::{TEST_EXACT_ARGUMENT, TEST_IGNORED_ARGUMENT};
use std::fs;
use std::process::Command;
use std::sync::Arc;

const CLEANUP_FIXTURE: &str =
    "integration_tests::process_descriptor_cleanup_tests::failed_spawn_cleanup_fixture";
const DESCRIPTOR_DIRECTORY: &str = "/dev/fd";
const MISSING_EXECUTABLE_NAME: &str = "missing-release-process";
const ERROR_CONTEXT: &str = "posix_spawn release command";
const CHILD_DESCRIPTOR: libc::c_int = 700;

#[test]
fn failed_spawn_releases_every_isolated_descriptor() {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([TEST_IGNORED_ARGUMENT, TEST_EXACT_ARGUMENT, CLEANUP_FIXTURE])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore]
fn failed_spawn_cleanup_fixture() {
    let fixture = Fixture::new();
    let source = Arc::new(tempfile::tempfile().unwrap());
    let missing_root = tempfile::tempdir().unwrap();
    let command = fixture
        .command(
            missing_root.path().join(MISSING_EXECUTABLE_NAME),
            Vec::new(),
            "failing spawn cleanup",
        )
        .inherit_as(source, CHILD_DESCRIPTOR)
        .with_input(Vec::new());
    let before = descriptor_count();
    let error = error_text(command.execute());
    assert!(error.contains(ERROR_CONTEXT), "{error}");
    assert_eq!(descriptor_count(), before);
}

fn descriptor_count() -> usize {
    fs::read_dir(DESCRIPTOR_DIRECTORY).unwrap().count()
}
