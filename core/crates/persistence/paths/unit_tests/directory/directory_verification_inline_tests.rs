use super::*;

#[test]
fn a_retained_leaf_reached_through_a_replaced_ancestor_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let parent = root.path().join("parent");
    let moved = root.path().join("moved");
    let leaf = PrivateDirectory::ensure(&parent.join("leaf")).unwrap();
    std::fs::rename(&parent, &moved).unwrap();
    std::os::unix::fs::symlink(&moved, &parent).unwrap();
    assert!(leaf.verify_path().is_err());
}

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
