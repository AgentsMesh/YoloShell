use crate::host::tests::*;
use super::*;

const LOCK_CONTENTION_OBSERVATION: Duration = Duration::from_millis(30);
const LOCK_COMPLETION_TIMEOUT: Duration = Duration::from_secs(1);
const HOST_LOCK_CONTENDERS: usize = 8;

#[tokio::test]
async fn host_socket_preparation_distinguishes_stale_and_live_sockets() {
    let directory = tempdir().unwrap();
    let socket = directory.path().join("custom.sock");
    let mut config = HostConfig::new(directory.path());
    config.socket = Some(socket.clone());
    let host = Host::open(config).unwrap();
    assert!(host.prepare_socket().await.is_ok());
    std::fs::write(&socket, b"stale").unwrap();
    assert!(host.prepare_socket().await.is_err());
    assert_eq!(std::fs::read(&socket).unwrap(), b"stale");
    std::fs::remove_file(&socket).unwrap();
    let listener = UnixListener::bind(&socket).unwrap();
    set_current_socket_permissions(&socket).unwrap();
    assert!(host.prepare_socket().await.is_err());
    drop(listener);
    host.prepare_socket().await.unwrap();
    assert!(!socket.exists());
    assert!(validate_socket_path(Path::new(&format!("/{}", "x".repeat(110)))).is_err());
    assert!(Host::open(HostConfig::new("/")).is_err());
}

#[test]
fn home_instance_lock_serializes_concurrent_startup_and_survives_stale_files() {
    use std::sync::Barrier;

    let directory = tempdir().unwrap();
    let home = directory.path().to_path_buf();
    let start = Arc::new(Barrier::new(HOST_LOCK_CONTENDERS));
    let finish = Arc::new(Barrier::new(HOST_LOCK_CONTENDERS));
    let handles: Vec<_> = (0..HOST_LOCK_CONTENDERS)
        .map(|_| {
            let home = home.clone();
            let start = Arc::clone(&start);
            let finish = Arc::clone(&finish);
            std::thread::spawn(move || {
                start.wait();
                let host = Host::open(HostConfig::new(home));
                let acquired = host.is_ok();
                finish.wait();
                drop(host);
                acquired
            })
        })
        .collect();
    let winners = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .filter(|acquired| *acquired)
        .count();
    assert_eq!(winners, 1);

    let lock_path = crate::coordination::lock_path(
        crate::coordination::CoordinationScope::HostHome,
        &home,
        None,
    )
    .unwrap();
    let host = Host::open(HostConfig::new(&home)).unwrap();
    assert_eq!(host.controller_generation(), 2);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&lock_path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let rejected = Host::open(HostConfig::new(&home)).err().unwrap();
    assert!(rejected.to_string().contains("already owns the home"));
    assert!(lock_path.exists());
    drop(host);
    let restarted = Host::open(HostConfig::new(&home)).unwrap();
    assert_eq!(restarted.controller_generation(), 3);
    assert!(lock_path.exists());
    drop(restarted);
}

#[test]
fn active_instance_lock_cannot_be_selected_as_the_host_socket() {
    let directory = tempdir().unwrap();
    let lock_path = crate::coordination::lock_path(
        crate::coordination::CoordinationScope::HostHome,
        directory.path(),
        None,
    )
    .unwrap();
    let mut config = HostConfig::new(directory.path());
    config.socket = Some(lock_path.clone());
    let error = Host::open(config).err().unwrap();
    assert!(
        error.to_string().contains("cannot replace"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn endpoint_lock_serializes_different_homes_using_one_custom_socket() {
    let directory = tempdir().unwrap();
    let socket_parent = directory.path().join("shared");
    std::fs::create_dir(&socket_parent).unwrap();
    let socket = socket_parent.join("host.sock");
    let mut first_config = HostConfig::new(directory.path().join("home-a"));
    first_config.socket = Some(socket.clone());
    let first = Host::open(first_config).unwrap();
    let mut second_config = HostConfig::new(directory.path().join("home-b"));
    second_config.socket = Some(socket.clone());
    let rejected = Host::open(second_config.clone()).err().unwrap();
    assert!(rejected.to_string().contains("already owns"));
    drop(first);
    let second = Host::open(second_config).unwrap();
    assert_eq!(second.controller_generation(), 1);
    assert!(socket_lock_path(&socket).unwrap().is_file());
}

#[tokio::test]
async fn session_locks_serialize_one_session_without_blocking_another() {
    let locks = Arc::new(SessionLocks::default());
    let first_id = Uuid::new_v4();
    let second_id = Uuid::new_v4();
    let first_guard = locks.lock(first_id).await;
    let waiting_locks = Arc::clone(&locks);
    let mut waiting = tokio::spawn(async move {
        let _guard = waiting_locks.lock(first_id).await;
    });
    assert!(
        tokio::time::timeout(LOCK_CONTENTION_OBSERVATION, &mut waiting)
            .await
            .is_err()
    );
    let second_guard = tokio::time::timeout(LOCK_CONTENTION_OBSERVATION, locks.lock(second_id))
        .await
        .unwrap();
    drop(second_guard);
    drop(first_guard);
    tokio::time::timeout(LOCK_COMPLETION_TIMEOUT, waiting)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn custom_host_socket_is_private_and_cleanup_cannot_unlink_a_replacement() {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let directory = tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let socket = paths.runtime_dir().join("custom.sock");
    let mut config = HostConfig::new(directory.path());
    config.socket = Some(socket.clone());
    let host = Host::open(config).unwrap();
    host.prepare_socket().await.unwrap();
    let not_a_socket = paths.runtime_dir().join("regular-file");
    std::fs::write(&not_a_socket, b"data").unwrap();
    assert!(SocketCleanup::new(&not_a_socket).is_err());
    let original = UnixListener::bind(&socket).unwrap();
    set_current_socket_permissions(&socket).unwrap();
    let cleanup = SocketCleanup::new(&socket).unwrap();
    #[cfg(unix)]
    {
        assert_eq!(
            std::fs::metadata(&socket).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(paths.runtime_dir())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }

    std::fs::remove_file(&socket).unwrap();
    let replacement = UnixListener::bind(&socket).unwrap();
    drop(cleanup);
    assert!(socket.exists());
    drop(replacement);
    std::fs::remove_file(&socket).unwrap();
    drop(original);
}
