use crate::worker::tests::*;
const TEST_IMMEDIATE_PROBE_TIMEOUT: Duration = Duration::ZERO;
const PROBE_TEST_WORKER_SOCKET_NAME: &str = "worker.sock";

fn provider_runtime() -> std::sync::Arc<yolo_agent_providers::ProviderRuntimeRegistry> {
    std::sync::Arc::new(yolo_agent_providers::builtin_provider_runtime())
}

#[tokio::test]
async fn worker_socket_probe_deadline_and_indeterminate_error_are_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(PROBE_TEST_WORKER_SOCKET_NAME);
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_worker_socket_permissions(&path).unwrap();
    let identity = worker_socket_identity(&path).unwrap();

    let timed_out = bounded_worker_socket_probe(
        std::future::pending::<io::Result<()>>(),
        TEST_IMMEDIATE_PROBE_TIMEOUT,
    )
    .await;
    assert!(matches!(timed_out, WorkerSocketProbe::TimedOut));
    let timeout_error = finish_worker_socket_probe(&path, identity, timed_out).unwrap_err();
    assert!(
        timeout_error
            .to_string()
            .contains(WORKER_SOCKET_PROBE_TIMEOUT_CONTEXT)
    );

    let denied = std::future::ready(Err::<(), _>(io::Error::from(
        io::ErrorKind::PermissionDenied,
    )));
    let indeterminate = bounded_worker_socket_probe(denied, WORKER_SOCKET_PROBE_TIMEOUT).await;
    let probe_error = finish_worker_socket_probe(&path, identity, indeterminate).unwrap_err();
    assert!(
        probe_error
            .to_string()
            .contains(WORKER_SOCKET_INDETERMINATE_CONTEXT)
    );
    assert_eq!(worker_socket_identity(&path).unwrap(), identity);
    drop(listener);
}

#[tokio::test]
async fn worker_socket_probe_distinguishes_live_and_refused_endpoints() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join(PROBE_TEST_WORKER_SOCKET_NAME);
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    set_current_worker_socket_permissions(&path).unwrap();
    let identity = worker_socket_identity(&path).unwrap();

    let live = bounded_worker_socket_probe(
        std::future::ready(Ok::<(), io::Error>(())),
        WORKER_SOCKET_PROBE_TIMEOUT,
    )
    .await;
    assert!(matches!(live, WorkerSocketProbe::Live));
    assert!(finish_worker_socket_probe(&path, identity, live).is_err());
    assert_eq!(worker_socket_identity(&path).unwrap(), identity);

    let refused = bounded_worker_socket_probe(
        std::future::ready(Err::<(), _>(io::ErrorKind::ConnectionRefused.into())),
        WORKER_SOCKET_PROBE_TIMEOUT,
    )
    .await;
    assert!(matches!(refused, WorkerSocketProbe::Refused));
    finish_worker_socket_probe(&path, identity, refused).unwrap();
    assert!(!path.exists());
    drop(listener);
}

#[test]
fn worker_startup_admission_is_serialized_with_lifecycle_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let launch = launch_spec(&paths, terminal_id);
    write_launch(&paths, terminal_id, &launch);
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();
    let lifecycle_lock = paths
        .try_acquire_session_lifecycle_lock(terminal_id)
        .unwrap()
        .unwrap();

    let error = prepare_worker_startup(&namespace, &paths, terminal_id, provider_runtime())
        .err()
        .unwrap();
    assert!(error.to_string().contains(STARTUP_ADMISSION_LOCK_CONTEXT));
    assert!(!paths.worker_startup_checkpoint(terminal_id).exists());
    drop(lifecycle_lock);

    let admitted = prepare_worker_startup(&namespace, &paths, terminal_id, provider_runtime()).unwrap();
    assert!(paths.worker_startup_checkpoint(terminal_id).is_file());
    assert!(
        !startup_checkpoint::worker_startup_checkpoint_allows_lifecycle_mutation(
            &paths,
            terminal_id,
            None,
        )
        .unwrap()
    );
    drop(admitted);
}

#[test]
fn worker_startup_admission_rejects_an_archived_manifest() {
    let directory = tempfile::tempdir().unwrap();
    let paths = Paths::new(directory.path());
    let terminal_id = Uuid::new_v4();
    let launch = launch_spec(&paths, terminal_id);
    write_launch(&paths, terminal_id, &launch);
    let manifest_path = paths.manifest(terminal_id);
    let mut manifest: crate::model::TerminalManifest = read_json(&manifest_path).unwrap();
    manifest.archived = true;
    atomic_write_json(&manifest_path, &manifest).unwrap();
    let namespace = WorkerNamespace::capture(&paths, terminal_id).unwrap();

    let error = prepare_worker_startup(&namespace, &paths, terminal_id, provider_runtime())
        .err()
        .unwrap();
    assert!(error.to_string().contains(STARTUP_ADMISSION_REJECTED));
    assert!(!paths.worker_startup_checkpoint(terminal_id).exists());
}
