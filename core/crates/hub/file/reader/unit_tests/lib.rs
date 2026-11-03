use crate::descriptor::io_error;
use yolo_hub_terminal_protocol::PublicError;

#[test]
fn filesystem_errors_keep_missing_unsafe_and_uncertain_outcomes_distinct() {
    for (raw, expected) in [
        (libc::ELOOP, PublicError::Unauthorized),
        (libc::ENOTDIR, PublicError::Unauthorized),
        (libc::ENOENT, PublicError::NotFound),
        (libc::EACCES, PublicError::Unauthorized),
        (libc::EPERM, PublicError::Unauthorized),
        (libc::EIO, PublicError::Unreachable),
        (libc::EMFILE, PublicError::Unreachable),
    ] {
        assert_eq!(io_error(std::io::Error::from_raw_os_error(raw)), expected);
    }
    assert_eq!(
        io_error(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)),
        PublicError::Conflict
    );
    assert_eq!(
        io_error(std::io::Error::from(std::io::ErrorKind::Other)),
        PublicError::Unreachable
    );
}
