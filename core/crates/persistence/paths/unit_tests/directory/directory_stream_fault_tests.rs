use super::*;
use std::fs::File;
use std::os::fd::{AsRawFd, RawFd};

const INVALID_DESCRIPTOR: RawFd = -1;

#[test]
fn stream_open_classifies_openat_and_fdopendir_failures() {
    let root = tempfile::tempdir().unwrap();
    let directory = File::open(root.path()).unwrap();
    assert!(DirectoryStream::open(INVALID_DESCRIPTOR).is_err());

    fail_next_stream_open_for_test();
    assert!(DirectoryStream::open(directory.as_raw_fd()).is_err());

    let mut recovered = DirectoryStream::open(directory.as_raw_fd()).unwrap();
    assert!(recovered.next_name().unwrap().is_none());
}

#[test]
fn stream_read_reports_an_injected_directory_error() {
    let root = tempfile::tempdir().unwrap();
    let directory = File::open(root.path()).unwrap();
    let mut stream = DirectoryStream::open(directory.as_raw_fd()).unwrap();

    fail_next_stream_read_for_test();
    assert!(stream.next_name().is_err());
    assert!(stream.next_name().unwrap().is_none());
}
