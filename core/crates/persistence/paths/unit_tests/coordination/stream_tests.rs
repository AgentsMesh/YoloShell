use super::*;
use std::os::fd::{AsRawFd, RawFd};
use std::ptr;

const INVALID_DIRECTORY_DESCRIPTOR: RawFd = -1;
const INJECTED_READ_ERROR: libc::c_int = libc::EIO;

#[test]
fn open_reports_invalid_directory_and_stream_conversion_failures() {
    assert!(DirectoryStream::open(INVALID_DIRECTORY_DESCRIPTOR).is_err());
    let temporary = tempfile::tempdir().unwrap();
    let directory = std::fs::File::open(temporary.path()).unwrap();

    assert!(DirectoryStream::open_with(directory.as_raw_fd(), |_| ptr::null_mut()).is_err());
}

#[test]
fn read_reports_errno_and_end_of_stream_independently() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = std::fs::File::open(temporary.path()).unwrap();
    let mut stream = DirectoryStream::open(directory.as_raw_fd()).unwrap();
    let error = stream
        .next_name_with(|_| {
            set_errno(INJECTED_READ_ERROR);
            ptr::null_mut()
        })
        .unwrap_err();
    assert_eq!(
        error
            .downcast_ref::<std::io::Error>()
            .unwrap()
            .raw_os_error(),
        Some(INJECTED_READ_ERROR)
    );
    assert_eq!(stream.next_name_with(|_| ptr::null_mut()).unwrap(), None);
}
