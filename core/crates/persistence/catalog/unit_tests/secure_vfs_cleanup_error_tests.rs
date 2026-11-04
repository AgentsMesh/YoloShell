#[test]
fn unsafe_shapes_are_preserved_but_io_failures_propagate() {
    for code in [
        libc::ENOENT,
        libc::EACCES,
        libc::ELOOP,
        libc::EISDIR,
        libc::ENOTEMPTY,
    ] {
        assert!(super::secure_vfs_cleanup_error::preserve_errno(code));
    }
    for code in [libc::EIO, libc::EMFILE] {
        assert!(!super::secure_vfs_cleanup_error::preserve_errno(code));
    }
    assert!(super::secure_vfs_cleanup_error::preserve(&anyhow::anyhow!(
        "semantic alias identity mismatch"
    )));
}

#[test]
fn only_lock_contention_is_treated_as_busy() {
    assert!(super::secure_vfs_cleanup_error::lock_errno(libc::EWOULDBLOCK).unwrap());
    assert!(super::secure_vfs_cleanup_error::lock_errno(libc::EIO).is_err());
}
