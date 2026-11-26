use crate::worker::tests::*;
const WORKER_SOCKET_REPLACEMENT_MODE: u32 = 0o700;
const SIMULATED_FOREIGN_UID_OFFSET: libc::uid_t = 1;
const REGULAR_SOCKET_PATH_BYTES: &[u8] = b"preserve-regular-worker-path";
const TEST_WORKER_SOCKET_NAME: &str = "worker.sock";

#[tokio::test]
async fn preexisting_regular_and_unsafe_mode_paths_are_preserved() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let regular_session = Uuid::new_v4();
    paths.ensure_session(regular_session).unwrap();
    let namespace = paths.retain_socket_namespace().unwrap();
    let regular_path = paths.worker_socket(regular_session).unwrap();
    std::fs::write(&regular_path, REGULAR_SOCKET_PATH_BYTES).unwrap();
    std::fs::set_permissions(
        &regular_path,
        std::fs::Permissions::from_mode(crate::paths::constants::PRIVATE_FILE_MODE),
    )
    .unwrap();
    let regular_limit = u64::try_from(REGULAR_SOCKET_PATH_BYTES.len()).unwrap();
    let regular_identity = crate::paths::identity::FileIdentity::inspect_bounded(
        &std::fs::symlink_metadata(&regular_path).unwrap(),
        &regular_path,
        regular_limit,
    )
    .unwrap();
    assert!(
        bind_worker_socket(&paths, regular_session, &regular_path)
            .await
            .is_err()
    );
    assert_eq!(
        std::fs::read(&regular_path).unwrap(),
        REGULAR_SOCKET_PATH_BYTES
    );

    let unsafe_session = Uuid::new_v4();
    paths.ensure_session(unsafe_session).unwrap();
    let unsafe_path = paths.worker_socket(unsafe_session).unwrap();
    let listener = std::os::unix::net::UnixListener::bind(&unsafe_path).unwrap();
    std::fs::set_permissions(
        &unsafe_path,
        std::fs::Permissions::from_mode(WORKER_SOCKET_REPLACEMENT_MODE),
    )
    .unwrap();
    let expected = worker_bound_socket_identity(&unsafe_path).unwrap();
    assert!(
        bind_worker_socket(&paths, unsafe_session, &unsafe_path)
            .await
            .is_err()
    );
    assert_eq!(
        worker_bound_socket_identity(&unsafe_path).unwrap(),
        expected
    );
    drop(listener);
    set_worker_socket_permissions(&unsafe_path, expected).unwrap();
    remove_stale_worker_socket(&unsafe_path, expected).unwrap();
    let current = crate::paths::identity::FileIdentity::inspect_bounded(
        &std::fs::symlink_metadata(&regular_path).unwrap(),
        &regular_path,
        regular_limit,
    )
    .unwrap();
    assert!(current.unchanged(regular_identity));
    std::fs::remove_file(&regular_path).unwrap();
    drop(namespace);
    assert!(!paths.socket_dir().unwrap().exists());
}

#[test]
fn owner_validation_has_a_simulated_foreign_uid_branch() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(TEST_WORKER_SOCKET_NAME);
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_worker_socket_permissions(&path).unwrap();
    let foreign_uid = unsafe { libc::geteuid() }.wrapping_add(SIMULATED_FOREIGN_UID_OFFSET);

    let error = worker_socket_identity_for_owner(&path, foreign_uid).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert!(error.to_string().contains(WORKER_SOCKET_OWNER_ERROR));
    assert!(worker_socket_identity(&path).is_ok());
    drop(listener);
}

#[test]
fn before_chmod_replacement_is_not_modified() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(TEST_WORKER_SOCKET_NAME);
    let original = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let original_identity = worker_bound_socket_identity(&path).unwrap();
    let replacement = std::cell::RefCell::new(None);
    let error = set_worker_socket_permissions_with_hook(&path, original_identity, || {
        std::fs::remove_file(&path)?;
        let listener = std::os::unix::net::UnixListener::bind(&path)?;
        std::fs::set_permissions(
            &path,
            std::fs::Permissions::from_mode(WORKER_SOCKET_REPLACEMENT_MODE),
        )?;
        replacement.replace(Some(listener));
        Ok(())
    })
    .err()
    .unwrap();

    assert!(error.to_string().contains(WORKER_SOCKET_CHANGED_CONTEXT));
    assert_ne!(
        worker_bound_socket_identity(&path).unwrap(),
        original_identity
    );
    assert_eq!(
        std::fs::symlink_metadata(&path).unwrap().mode() & WORKER_SOCKET_PERMISSION_MASK,
        WORKER_SOCKET_REPLACEMENT_MODE
    );
    drop(replacement);
    drop(original);
}

#[test]
fn refused_probe_cannot_remove_a_replacement_socket() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(TEST_WORKER_SOCKET_NAME);
    let stale = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_worker_socket_permissions(&path).unwrap();
    let stale_identity = worker_socket_identity(&path).unwrap();
    drop(stale);
    let replacement = std::cell::RefCell::new(None);

    let error = finish_worker_socket_probe_with_hook(
        &path,
        stale_identity,
        WorkerSocketProbe::Refused,
        || {
            std::fs::remove_file(&path)?;
            let listener = std::os::unix::net::UnixListener::bind(&path)?;
            set_current_worker_socket_permissions(&path)?;
            replacement.replace(Some(listener));
            Ok(())
        },
    )
    .err()
    .unwrap();

    assert!(error.to_string().contains(WORKER_SOCKET_CHANGED_CONTEXT));
    assert_ne!(worker_socket_identity(&path).unwrap(), stale_identity);
    drop(replacement);
}
