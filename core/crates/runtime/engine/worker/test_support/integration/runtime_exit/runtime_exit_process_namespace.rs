#[cfg(target_os = "macos")]
fn assert_orphan_guardian_uses_explicit_namespace(paths: &Paths, terminal_id: Uuid) {
    use std::os::unix::fs::FileTypeExt;

    let custody: crate::pty_custody::CustodyDurableRecord =
        read_json(&paths.pty_custody(terminal_id)).unwrap();
    let guardian = custody.armed_lease().unwrap().guardian.unwrap();
    let socket = paths
        .custody_guardian_socket(guardian.guardian_id())
        .unwrap();
    let namespace = paths.socket_dir().unwrap();
    assert_eq!(socket.parent(), Some(namespace.as_path()));
    assert_ne!(
        namespace.parent().unwrap().as_os_str(),
        std::env::var_os(crate::paths::SOCKET_ROOT_ENVIRONMENT)
            .unwrap()
            .as_os_str()
    );
    assert!(
        std::fs::symlink_metadata(socket)
            .unwrap()
            .file_type()
            .is_socket()
    );
}
