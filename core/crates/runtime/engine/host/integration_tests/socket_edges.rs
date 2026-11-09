use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn stale_socket_directory_and_missing_permissions_are_reported() {
    let directory = tempdir().unwrap();
    let socket = directory.path().join("directory.sock");
    let mut config = HostConfig::new(directory.path());
    config.socket = Some(socket.clone());
    let host = Host::open(config).unwrap();
    std::fs::create_dir(&socket).unwrap();
    assert!(host.prepare_socket().await.is_err());
    assert!(set_current_socket_permissions(&directory.path().join("missing.sock")).is_err());
}

#[test]
fn instance_lock_rejects_non_regular_targets() {
    let directory = tempdir().unwrap();
    assert!(InstanceLock::acquire(directory.path(), "test resource").is_err());
    assert!(directory.path().is_dir());
}

#[test]
fn instance_lock_rejects_hardlinks_without_changing_the_target() {
    use std::os::unix::fs::PermissionsExt;
    const TARGET_BYTES: &[u8] = b"preserve-lock-target";
    let directory = tempdir().unwrap();
    let target = directory.path().join("target");
    let lock = directory.path().join("host.lock");
    std::fs::write(&target, TARGET_BYTES).unwrap();
    std::fs::set_permissions(
        &target,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    std::fs::hard_link(&target, &lock).unwrap();

    assert!(InstanceLock::acquire(&lock, "test resource").is_err());
    assert_eq!(std::fs::read(target).unwrap(), TARGET_BYTES);
}

#[test]
fn instance_lock_rejects_path_replacement_after_flock() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempdir().unwrap();
    let path = directory.path().join("host.lock");
    let error = InstanceLock::acquire_with_hook(&path, "test resource", || {
        std::fs::remove_file(&path)?;
        std::fs::write(&path, [])?;
        std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
        )?;
        Ok(())
    })
    .err()
    .unwrap();
    assert!(
        error
            .to_string()
            .contains("verify locked private file path")
    );
    assert!(InstanceLock::acquire(&path, "test resource").is_ok());
}

#[cfg(target_os = "macos")]
#[test]
fn coordination_parent_replacement_cannot_create_a_second_host_lock() {
    let root = tempdir().unwrap();
    let original = root.path().join("coordination");
    let displaced = root.path().join("displaced-coordination");
    crate::paths::directory::PrivateDirectory::ensure(&original).unwrap();
    let path = original.join(format!("host-{}.lock", Uuid::new_v4()));
    let first = InstanceLock::acquire(&path, "test resource").unwrap();
    std::fs::rename(&original, &displaced).unwrap();
    crate::paths::directory::PrivateDirectory::ensure(&original).unwrap();

    assert!(InstanceLock::acquire(&path, "test resource").is_err());
    drop(first);
    assert!(InstanceLock::acquire(&path, "test resource").is_ok());
}
