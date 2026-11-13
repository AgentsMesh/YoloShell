use super::capture;
use crate::Error;
use std::{fs::File, os::fd::AsFd};

const PROC_DIRECTORY: &str = "/proc/self";

#[test]
fn procfs_cannot_issue_an_export_handle() {
    let directory = File::open(PROC_DIRECTORY).unwrap();
    assert!(matches!(
        capture(directory.as_fd()),
        Err(Error::Unsupported(_))
    ));
}
