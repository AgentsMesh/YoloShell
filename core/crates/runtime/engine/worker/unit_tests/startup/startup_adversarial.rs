use crate::worker::tests::*;
#[tokio::test]
async fn duplicate_worker_launch_cannot_spawn_a_second_pty() {
    let directory = GuardianRuntimeDirectory::new();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let marker = directory.path().join("child-started.marker");
    let launch = adversarial_launch(&paths, terminal_id, &marker);
    write_launch(&paths, terminal_id, &launch);
    write_adversarial_manifest(&paths, terminal_id);
    let custody = spawn_active_runtime_custody(paths.clone(), terminal_id);
    let mut first = tokio::spawn(run(paths.clone(), terminal_id));
    let record = wait_for_adversarial_worker(&paths, terminal_id, &marker, &mut first).await;
    #[cfg(target_os = "macos")]
    let guardian =
        read_json::<crate::pty_custody::CustodyDurableRecord>(&paths.pty_custody(terminal_id))
            .unwrap()
            .armed_lease()
            .unwrap()
            .guardian
            .unwrap()
            .process();
    assert!(!paths.worker_launch(terminal_id).exists());
    let mut child_cleanup = TestProcessCleanup(Some(process::ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    }));

    write_launch(&paths, terminal_id, &launch);
    let socket_before = worker_socket_identity(&launch.socket_path).unwrap();
    let duplicate_error = run(paths.clone(), terminal_id).await.unwrap_err();
    assert!(
        duplicate_error
            .to_string()
            .contains(WORKER_INSTANCE_BUSY_CONTEXT)
    );
    assert_eq!(
        worker_socket_identity(&launch.socket_path).unwrap(),
        socket_before
    );
    tokio::time::sleep(TEST_DUPLICATE_OBSERVATION_DELAY).await;
    assert_eq!(std::fs::read(&marker).unwrap(), TEST_MARKER_CONTENT);
    retire_adversarial_launch(&paths, terminal_id);

    let response = rpc(
        &record,
        record.controller_generation,
        crate::protocol::method::TERMINATE_RUNTIME_EXACT,
        &FencedTerminateRequest {
            terminal_id,
            runtime_epoch: record.runtime_epoch,
            expected_status: None,
            force: true,
        },
    )
    .await
    .unwrap();
    let cleanup_deferred = response
        .error
        .as_ref()
        .is_some_and(|error| error.code == crate::protocol::error_code::CLEANUP_DEFERRED);
    assert!(
        response.ok || cleanup_deferred,
        "duplicate worker terminate response: {:?}",
        response.error
    );
    custody.await.unwrap().unwrap();
    assert!(
        tokio::time::timeout(TEST_SHUTDOWN_LIFECYCLE_TIMEOUT, first)
            .await
            .unwrap()
            .unwrap()
            .is_ok()
    );
    let exit: ExitRecord = read_json(&paths.exit_record(terminal_id)).unwrap();
    assert_eq!(exit.runtime_epoch, record.runtime_epoch);
    assert!(
        !process::ProcessIdentity {
            pid: record.child_pid,
            start_time: record.child_start_time,
        }
        .matches()
    );
    child_cleanup.0 = None;
    let namespace_path = paths.socket_dir().unwrap();
    directory.close(&paths).unwrap();
    assert!(!namespace_path.exists());
    #[cfg(target_os = "macos")]
    assert_eq!(
        guardian.observation(),
        crate::pty_custody::CustodyHolderObservation::ProvenGone
    );
}

async fn fill_frozen_backlog(listener: &UnixListener, path: &Path) -> Vec<UnixStream> {
    use std::os::fd::AsRawFd;
    assert_eq!(
        unsafe { libc::listen(listener.as_raw_fd(), TEST_LISTEN_BACKLOG) },
        0
    );
    let mut clients = Vec::new();
    for _ in 0..TEST_BACKLOG_ATTEMPTS {
        match tokio::time::timeout(TEST_BACKLOG_CONNECT_TIMEOUT, UnixStream::connect(path)).await {
            Ok(Ok(client)) => clients.push(client),
            Err(_) => return clients,
            Ok(Err(error)) if error.kind() == io::ErrorKind::ConnectionRefused => return clients,
            #[cfg(target_os = "linux")]
            Ok(Err(error)) if error.kind() == io::ErrorKind::WouldBlock => return clients,
            Ok(Err(error)) => panic!("fill frozen worker backlog: {error}"),
        }
    }
    panic!("worker socket backlog did not saturate");
}

#[tokio::test]
async fn frozen_full_backlog_is_bounded_and_never_unlinked() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    paths.ensure_session(terminal_id).unwrap();
    let path = paths.worker_socket(terminal_id).unwrap();
    let (listener, _owner) = bind_worker_socket(&paths, terminal_id, &path).await.unwrap();
    let _clients = fill_frozen_backlog(&listener, &path).await;
    let expected = worker_socket_identity(&path).unwrap();
    let error = bind_worker_socket(&paths, terminal_id, &path)
        .await
        .err()
        .unwrap();
    assert!(error.to_string().contains(WORKER_INSTANCE_BUSY_CONTEXT));
    assert_eq!(worker_socket_identity(&path).unwrap(), expected);
}

#[test]
fn stale_inode_cannot_authorize_unlinking_a_replacement_socket() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("worker.sock");
    let stale = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_worker_socket_permissions(&path).unwrap();
    let stale_identity = worker_socket_identity(&path).unwrap();
    drop(stale);
    std::fs::remove_file(&path).unwrap();
    let replacement = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_worker_socket_permissions(&path).unwrap();
    let replacement_identity = worker_socket_identity(&path).unwrap();
    assert_ne!(replacement_identity, stale_identity);
    assert!(remove_stale_worker_socket(&path, stale_identity).is_err());
    assert_eq!(worker_socket_identity(&path).unwrap(), replacement_identity);
    drop(replacement);
}
