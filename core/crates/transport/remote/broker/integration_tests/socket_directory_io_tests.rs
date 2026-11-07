use super::*;
use std::ffi::CString;
use std::fs::File;
use std::os::fd::AsRawFd;

const MISSING_SOURCE: &str = "missing.sock";
const MISSING_TARGET: &str = "target.sock";

#[test]
fn missing_entries_report_unlink_and_rename_failures() {
    let root = tempfile::tempdir().unwrap();
    let directory = File::open(root.path()).unwrap();
    let source = CString::new(MISSING_SOURCE).unwrap();
    let target = CString::new(MISSING_TARGET).unwrap();

    assert!(unlink(directory.as_raw_fd(), &source).is_err());
    assert!(
        rename_exclusive(
            directory.as_raw_fd(),
            &source,
            directory.as_raw_fd(),
            &target,
        )
        .is_err()
    );
}
