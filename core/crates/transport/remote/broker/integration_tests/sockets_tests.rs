use super::super::sockets::BrokerSockets;
use super::super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tokio::net::UnixListener;

const LOCK_TARGET_BYTES: &[u8] = b"preserve-instance-lock-target";

#[tokio::test]
async fn stale_private_sockets_are_replaced_and_cleaned() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    let stale = UnixListener::bind(paths.runtime_socket()).unwrap();
    fs::set_permissions(paths.runtime_socket(), fs::Permissions::from_mode(0o600)).unwrap();
    drop(stale);
    let sockets = BrokerSockets::bind(&paths).await.unwrap();
    assert!(validate_broker_socket(&paths.runtime_socket()).is_ok());
    assert!(validate_broker_socket(&paths.management_socket()).is_ok());
    drop(sockets);
    assert!(!paths.runtime_socket().exists());
}

#[tokio::test]
async fn cleanup_preserves_socket_replaced_after_binding() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    let sockets = BrokerSockets::bind(&paths).await.unwrap();
    fs::remove_file(paths.runtime_socket()).unwrap();
    let replacement = UnixListener::bind(paths.runtime_socket()).unwrap();
    fs::set_permissions(
        paths.runtime_socket(),
        fs::Permissions::from_mode(super::super::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    drop(sockets);
    assert!(paths.runtime_socket().exists());
    drop(replacement);
}

#[tokio::test]
async fn unsafe_lock_types_fail_closed_without_touching_target() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    fs::create_dir(paths.instance_lock()).unwrap();
    assert!(BrokerSockets::bind(&paths).await.is_err());
    assert!(paths.instance_lock().is_dir());
}

#[tokio::test]
async fn instance_lock_rejects_hardlinks_without_mutating_the_target() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    let target = paths.state_dir().join("lock-target");
    fs::write(&target, LOCK_TARGET_BYTES).unwrap();
    fs::set_permissions(
        &target,
        fs::Permissions::from_mode(super::super::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    fs::hard_link(&target, paths.instance_lock()).unwrap();

    assert!(BrokerSockets::bind(&paths).await.is_err());
    assert_eq!(fs::read(target).unwrap(), LOCK_TARGET_BYTES);
}

#[tokio::test]
async fn instance_lock_rejects_path_replacement_after_flock() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    let lock_path = paths.instance_lock();
    let error = BrokerSockets::bind_with_lock_hook(&paths, || {
        fs::remove_file(lock_path)?;
        fs::write(lock_path, [])?;
        fs::set_permissions(
            lock_path,
            fs::Permissions::from_mode(super::super::constants::PRIVATE_FILE_MODE),
        )?;
        Ok(())
    })
    .await
    .err()
    .unwrap();

    assert!(
        error
            .to_string()
            .contains("verify locked private file path")
    );
    assert!(BrokerSockets::bind(&paths).await.is_ok());
}

#[tokio::test]
async fn malicious_endpoint_and_duplicate_instance_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path().join("state")).unwrap();
    fs::write(paths.runtime_socket(), b"do-not-remove").unwrap();
    fs::set_permissions(paths.runtime_socket(), fs::Permissions::from_mode(0o600)).unwrap();
    assert!(BrokerSockets::bind(&paths).await.is_err());
    assert_eq!(fs::read(paths.runtime_socket()).unwrap(), b"do-not-remove");
    fs::remove_file(paths.runtime_socket()).unwrap();

    let first = BrokerSockets::bind(&paths).await.unwrap();
    assert!(BrokerSockets::bind(&paths).await.is_err());
    drop(first);
}

#[test]
fn broker_socket_validation_rejects_relative_regular_and_public_socket() {
    assert!(validate_broker_socket_path(std::path::Path::new("relative.sock")).is_err());
    let root = tempfile::tempdir().unwrap();
    let regular = root.path().join("regular.sock");
    fs::write(&regular, b"file").unwrap();
    assert!(validate_broker_socket(&regular).is_err());
    fs::remove_file(&regular).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&regular).unwrap();
    fs::set_permissions(&regular, fs::Permissions::from_mode(0o666)).unwrap();
    assert!(validate_broker_socket(&regular).is_err());
    drop(listener);
}
