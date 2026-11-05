use super::*;

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn retained_path_observation_distinguishes_loss_from_io_uncertainty() {
    assert!(matches!(
        classify_path_observation(Ok(false)),
        RetainedPathVerification::IdentityLost
    ));
    for code in [libc::ENOENT, libc::ENOTDIR, libc::ELOOP] {
        let observed = Err(std::io::Error::from_raw_os_error(code));
        assert!(matches!(
            classify_path_observation(observed),
            RetainedPathVerification::IdentityLost
        ));
    }
    for code in [libc::EIO, libc::EMFILE, libc::EACCES] {
        let observed = Err(std::io::Error::from_raw_os_error(code));
        assert!(matches!(
            classify_path_observation(observed),
            RetainedPathVerification::Indeterminate(_)
        ));
    }
}
