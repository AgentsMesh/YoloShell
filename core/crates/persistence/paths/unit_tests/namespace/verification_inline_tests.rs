use super::*;

const TEST_ANCHOR: &str = "verification-test-anchor";

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn ordinary_io_errors_remain_indeterminate_and_recoverable() {
    for code in [libc::EIO, libc::EMFILE, libc::EACCES] {
        let root = tempfile::tempdir().unwrap();
        let managed = root.path().join(TEST_ANCHOR);
        PrivateDirectory::ensure(&managed).unwrap();
        let lease = NamespaceLease::capture(&[(TEST_ANCHOR, &managed)]).unwrap();
        let mut injected = Some(RetainedPathVerification::Indeterminate(
            std::io::Error::from_raw_os_error(code).into(),
        ));

        assert!(matches!(
            lease.verification_with(|_| injected.take().unwrap()),
            NamespaceVerification::Indeterminate(_)
        ));
        assert!(!lease.is_lost());
        assert!(lease.prove_ownership_loss().is_none());
        assert!(lease.verify().is_ok());
    }
}
