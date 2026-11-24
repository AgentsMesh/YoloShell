use super::*;
use std::ffi::CString;
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd};

#[path = "harness_namespace_retirement_test_io.rs"]
mod test_io;
use test_io::TestIo;

const RETAINED_WRITE_NAME: &str = "retained-write";
const RETAINED_WRITE_CONTENT: &[u8] = b"retained-directory-authority";
const RETAINED_WRITE_MODE: libc::c_uint = 0o600;
const RETAINED_WRITE_FLAGS: libc::c_int =
    libc::O_CLOEXEC | libc::O_CREAT | libc::O_TRUNC | libc::O_WRONLY;
const INVALID_DESCRIPTOR: libc::c_int = -1;
const INJECTED_RENAME_FAILURE: &str = "injected namespace rename failure";
const INJECTED_REMOVE_FAILURE: &str = "injected namespace removal failure";
const TRANSIENT_REMOVE_FAILURES: usize = 2;
const EXPECTED_REMOVE_CALLS: usize = 3;
const EXPECTED_SINGLE_OCCURRENCE: usize = 1;

#[test]
fn retained_directory_descriptor_can_only_write_inside_tombstone() {
    let mut home = tempfile::tempdir().unwrap();
    let original = home.path().to_path_buf();
    let directory = File::open(&original).unwrap();
    let retirement = NamespaceRetirement::retire(&mut home);
    let tombstone = retirement.path_for_test().to_path_buf();

    assert!(!original.exists());
    write_at(&directory, RETAINED_WRITE_NAME, RETAINED_WRITE_CONTENT);
    assert_eq!(
        std::fs::read(tombstone.join(RETAINED_WRITE_NAME)).unwrap(),
        RETAINED_WRITE_CONTENT
    );
    assert!(!original.exists());

    retirement.finish().unwrap();
    assert!(!tombstone.exists());
}

#[test]
fn rename_failure_is_visible_and_original_path_is_explicitly_removed() {
    let mut home = tempfile::tempdir().unwrap();
    let original = home.path().to_path_buf();
    let io = TestIo::rename_failure();
    let retirement = NamespaceRetirement::retire_with(&mut home, &io);

    let detail = format!("{:#}", retirement.finish_with(&io).unwrap_err());
    assert!(detail.contains(RENAME_CONTEXT));
    assert!(detail.contains(INJECTED_RENAME_FAILURE));
    assert!(!original.exists());
}

#[test]
fn final_remove_failure_is_visible_once_and_leaves_the_tombstone_owned() {
    let mut home = tempfile::tempdir().unwrap();
    let io = TestIo::remove_failure();
    let retirement = NamespaceRetirement::retire_with(&mut home, &io);
    let tombstone = retirement.path_for_test().to_path_buf();

    let detail = format!("{:#}", retirement.finish_with(&io).unwrap_err());
    assert!(detail.contains(REMOVE_CONTEXT));
    assert_eq!(
        detail.matches(INJECTED_REMOVE_FAILURE).count(),
        EXPECTED_SINGLE_OCCURRENCE
    );
    assert_eq!(io.elapsed(), REMOVE_SETTLEMENT_TIMEOUT);
    assert!(tombstone.exists());
    std::fs::remove_dir_all(tombstone).unwrap();
}

#[test]
fn transient_remove_failures_are_retried_before_success() {
    let mut home = tempfile::tempdir().unwrap();
    let io = TestIo::transient_remove_failures(
        TRANSIENT_REMOVE_FAILURES,
        std::io::ErrorKind::DirectoryNotEmpty,
    );
    let retirement = NamespaceRetirement::retire_with(&mut home, &io);
    let tombstone = retirement.path_for_test().to_path_buf();

    retirement.finish_with(&io).unwrap();
    assert_eq!(io.remove_calls(), EXPECTED_REMOVE_CALLS);
    assert!(!tombstone.exists());
}

#[test]
fn interrupted_remove_uses_bounded_backoff_before_success() {
    let mut home = tempfile::tempdir().unwrap();
    let io = TestIo::transient_remove_failures(
        TRANSIENT_REMOVE_FAILURES,
        std::io::ErrorKind::Interrupted,
    );
    let retirement = NamespaceRetirement::retire_with(&mut home, &io);

    retirement.finish_with(&io).unwrap();
    assert_eq!(io.remove_calls(), EXPECTED_REMOVE_CALLS);
    assert!(io.elapsed() >= super::super::PROCESS_POLL_INTERVAL);
    assert!(io.elapsed() < REMOVE_SETTLEMENT_TIMEOUT);
}

#[test]
fn permanent_interrupted_remove_stops_at_deadline_with_one_root_cause() {
    let mut home = tempfile::tempdir().unwrap();
    let io = TestIo::permanent_remove_failure(std::io::ErrorKind::Interrupted);
    let retirement = NamespaceRetirement::retire_with(&mut home, &io);
    let tombstone = retirement.path_for_test().to_path_buf();

    let detail = format!("{:#}", retirement.finish_with(&io).unwrap_err());
    assert!(detail.contains(REMOVE_CONTEXT));
    assert_eq!(
        detail.matches(INJECTED_REMOVE_FAILURE).count(),
        EXPECTED_SINGLE_OCCURRENCE
    );
    assert_eq!(io.elapsed(), REMOVE_SETTLEMENT_TIMEOUT);
    assert!(tombstone.exists());
    std::fs::remove_dir_all(tombstone).unwrap();
}

fn write_at(directory: &File, name: &str, content: &[u8]) {
    let name = CString::new(name).unwrap();
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            RETAINED_WRITE_FLAGS,
            RETAINED_WRITE_MODE,
        )
    };
    assert_ne!(descriptor, INVALID_DESCRIPTOR);
    let mut file = unsafe { File::from_raw_fd(descriptor) };
    file.write_all(content).unwrap();
}
