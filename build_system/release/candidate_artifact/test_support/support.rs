use anyhow::Result;
use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use yolo_release_contracts::PRIVATE_FILE_MODE;

pub const ARCHIVE_BYTES: &[u8] = b"stable candidate archive\n";
pub const OTHER_ARCHIVE_BYTES: &[u8] = b"different candidate archive\n";

pub fn error_text<T>(result: Result<T>) -> String {
    format!("{:#}", result.err().expect("expected error"))
}

pub fn fifo(path: &Path) {
    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    assert_eq!(
        unsafe { libc::mkfifo(path.as_ptr(), PRIVATE_FILE_MODE as libc::mode_t) },
        0
    );
}
