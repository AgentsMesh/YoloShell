use crate::host::tests::*;
use super::*;

async fn replace_runtime_during_status(
    listener: UnixListener,
    authenticated: RuntimeRecord,
    path: PathBuf,
    exit_path: PathBuf,
    replacement: RuntimeRecord,
    _paths: Paths,
) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, &authenticated).await;
    let _: RequestEnvelope = stream.request().await.unwrap();
    atomic_write_json(&path, &replacement).unwrap();
    atomic_write_json(&exit_path, &archive_exit_for(replacement.runtime_epoch)).unwrap();
    #[cfg(target_os = "linux")]
    crate::host::linux_retirement_evidence::publish_paths(&_paths, &replacement);
}

#[tokio::test]
async fn convergence_inactive_settlement_classifies_every_runtime_state() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let missing = host
        .settle_inactive_bound_session(Uuid::new_v4(), false, None)
        .await
        .unwrap_err();
    assert_eq!(missing.code, crate::protocol::error_code::NOT_FOUND);

    let mut archived = fixture_manifest(directory.path());
    archived.archived = true;
    seed_recovery_manifest(&host, &archived).await;
    host.settle_inactive_bound_session(archived.terminal_id, false, None)
        .await
        .unwrap();

    for status in [TerminalRuntimeStatus::Exited, TerminalRuntimeStatus::Running] {
        let manifest = fixture_manifest(directory.path());
        let socket = lifecycle_convergence_socket(directory.path(), manifest.terminal_id);
        let listener = UnixListener::bind(&socket).unwrap();
        let record = host_runtime_record(&host, manifest.terminal_id, socket);
        let mut view = status_view(&record, Some(Vec::new()));
        view.status = status;
        persist_host_session(&host, &manifest, &record);
        let server = tokio::spawn(fake_status_server(listener, record, Some(view)));
        let result = host
            .settle_inactive_bound_session(manifest.terminal_id, false, None)
            .await;
        if status == TerminalRuntimeStatus::Exited {
            result.unwrap();
        } else {
            assert_eq!(
                result.unwrap_err().code,
                crate::protocol::error_code::CONFLICT
            );
        }
        server.await.unwrap();
    }

    let live = fixture_manifest(directory.path());
    let socket = lifecycle_convergence_socket(directory.path(), live.terminal_id);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, live.terminal_id, socket);
    persist_host_session(&host, &live, &record);
    let server = tokio::spawn(fake_status_server(listener, record, None));
    assert!(
        host.settle_inactive_bound_session(live.terminal_id, false, None)
            .await
            .is_err()
    );
    server.await.unwrap();

    let replaced = fixture_manifest(directory.path());
    let socket = lifecycle_convergence_socket(directory.path(), replaced.terminal_id);
    let listener = UnixListener::bind(&socket).unwrap();
    let record = host_runtime_record(&host, replaced.terminal_id, socket);
    let runtime_path = host.paths.runtime_record(replaced.terminal_id);
    let exit_path = host.paths.exit_record(replaced.terminal_id);
    persist_host_session(&host, &replaced, &record);
    let server = tokio::spawn(replace_runtime_during_status(
        listener,
        record.clone(),
        runtime_path,
        exit_path,
        lifecycle_convergence_dead_runtime(&record),
        host.paths.clone(),
    ));
    host.settle_inactive_bound_session(replaced.terminal_id, true, None)
        .await
        .unwrap();
    server.await.unwrap();

    let absent = fixture_manifest(directory.path());
    seed_recovery_manifest(&host, &absent).await;
    assert!(
        host.settle_inactive_bound_session(absent.terminal_id, false, None)
            .await
            .is_err()
    );
    let dead = fixture_manifest(directory.path());
    let record = lifecycle_convergence_dead_runtime(&host_runtime_record(
        &host,
        dead.terminal_id,
        lifecycle_convergence_socket(directory.path(), dead.terminal_id),
    ));
    persist_host_session(&host, &dead, &record);
    persist_archive_exit(&host, &record, record.runtime_epoch);
    host.settle_inactive_bound_session(dead.terminal_id, false, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn durable_child_exit_settles_without_worker_custody() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let runtime = lifecycle_convergence_dead_runtime(&host_runtime_record(
        &host,
        manifest.terminal_id,
        lifecycle_convergence_socket(directory.path(), manifest.terminal_id),
    ));
    persist_host_session(&host, &manifest, &runtime);
    persist_archive_exit(&host, &runtime, runtime.runtime_epoch);
    host.secondary_custody.revoke_rpc_test(&runtime);

    host.settle_inactive_bound_session(manifest.terminal_id, false, None)
        .await
        .unwrap();
    host.wait_for_session_exit(manifest.terminal_id)
        .await
        .unwrap();
}

#[tokio::test]
async fn convergence_exit_wait_distinguishes_absent_corrupt_and_live_runtime() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.wait_for_session_exit(Uuid::new_v4()).await.unwrap();

    let corrupt_id = Uuid::new_v4();
    host.paths.ensure_session(corrupt_id).unwrap();
    write_private_bytes(
        &host.paths.runtime_record(corrupt_id),
        LIFECYCLE_CONVERGENCE_CORRUPT_RUNTIME,
    );
    assert!(host.wait_for_session_exit(corrupt_id).await.is_err());

    for status in [TerminalRuntimeStatus::Exited, TerminalRuntimeStatus::Running] {
        let terminal_id = Uuid::new_v4();
        let socket = lifecycle_convergence_socket(directory.path(), terminal_id);
        let listener = UnixListener::bind(&socket).unwrap();
        let record = host_runtime_record(&host, terminal_id, socket);
        let mut view = status_view(&record, Some(Vec::new()));
        view.status = status;
        persist_archive_runtime(&host, &record);
        let server = tokio::spawn(fake_status_server(listener, record, Some(view)));
        let result = host.wait_for_session_exit(terminal_id).await;
        if status == TerminalRuntimeStatus::Exited {
            result.unwrap();
        } else {
            assert!(result.is_err());
        }
        server.await.unwrap();
    }
}
