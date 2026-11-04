use super::*;

const CLASSIFICATION_ERROR_EXPECTED: &str = "namespace capture classification must fail";
const HARD_CAPTURE_ERRNOS: [libc::c_int; 3] = [libc::EIO, libc::EACCES, libc::EMFILE];
const IDENTITY_LOSS_ERRNOS: [libc::c_int; 3] = [libc::ENOENT, libc::ENOTDIR, libc::ELOOP];

#[test]
fn definitive_namespace_capture_loss_has_terminal_authority_type() {
    for errno in IDENTITY_LOSS_ERRNOS {
        let source = std::io::Error::from_raw_os_error(errno);

        let error = classify_namespace_capture_failure(source.into())
            .err()
            .expect(CLASSIFICATION_ERROR_EXPECTED);

        assert!(error.is::<ManifestAuthorityUnavailable>());
    }
}

#[test]
fn hard_namespace_capture_failures_keep_their_io_classification() {
    for errno in HARD_CAPTURE_ERRNOS {
        let source = std::io::Error::from_raw_os_error(errno);
        let expected_kind = source.kind();

        let error = classify_namespace_capture_failure(source.into())
            .err()
            .expect(CLASSIFICATION_ERROR_EXPECTED);

        assert!(!error.is::<ManifestAuthorityUnavailable>());
        assert_eq!(
            crate::paths::private_file::io_kind(&error),
            Some(expected_kind)
        );
    }
}
