use crate::host::tests::*;
use super::*;

pub(super) async fn fake_status_server(
    listener: UnixListener,
    record: RuntimeRecord,
    status: Option<RuntimeView>,
) {
    let (stream, _) = listener.accept().await.unwrap();
    let mut stream = UnixStream::from(stream);
    let mut stream = accept_worker_auth(stream, &record).await;
    let request: RequestEnvelope = stream.request().await.unwrap();
    if let Some(status) = status {
        let response = ResponseEnvelope::success(&request, &status).unwrap();
        stream.respond(&response).await.unwrap();
    }
}

#[tokio::test]
async fn reconcile_classifies_exited_unreachable_and_missing_runtimes() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();

    let exited = fixture_manifest(directory.path());
    let exited_socket = directory.path().join("exited.sock");
    let listener = UnixListener::bind(&exited_socket).unwrap();
    let exited_record = host_runtime_record(&host, exited.terminal_id, exited_socket);
    persist_host_session(&host, &exited, &exited_record);
    let status = RuntimeView {
        presentation: Default::default(),
        runtime_epoch: exited_record.runtime_epoch,
        worker_id: exited_record.worker_id,
        worker_pid: exited_record.worker_pid,
        worker_start_time: exited_record.worker_start_time,
        child_pid: exited_record.child_pid,
        child_start_time: exited_record.child_start_time,
        status: TerminalRuntimeStatus::Exited,
        last_output_sequence: 0,
        exit: None,
        bootstrap_delivery: exited_record.bootstrap_delivery,
        capabilities: Some(Vec::new()),
        fault_reap: None,
    };
    let exited_server = tokio::spawn(fake_status_server(listener, exited_record, Some(status)));

    let unreachable = fixture_manifest(directory.path());
    let unreachable_socket = directory.path().join("unreachable.sock");
    let listener = UnixListener::bind(&unreachable_socket).unwrap();
    let unreachable_record = host_runtime_record(&host, unreachable.terminal_id, unreachable_socket);
    persist_host_session(&host, &unreachable, &unreachable_record);
    let unreachable_server = tokio::spawn(fake_status_server(listener, unreachable_record, None));

    let missing = fixture_manifest(directory.path());
    host.paths.ensure_session(missing.terminal_id).unwrap();
    host.catalog.lock().unwrap().insert(&missing).unwrap();
    atomic_write_json(&host.paths.manifest(missing.terminal_id), &missing).unwrap();

    let result = host.reconcile().await;
    assert_eq!(result.exited, 1);
    assert_eq!(result.unreachable, 2);
    assert_eq!(result.needs_recreate, 0);
    exited_server.await.unwrap();
    unreachable_server.await.unwrap();
}

#[tokio::test]
async fn reboot_with_a_persisted_exit_skips_automatic_restore() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join("old.sock"),
    );
    record.boot_id = "previous-boot".into();
    persist_host_session(&host, &manifest, &record);
    atomic_write_json(
        &host.paths.exit_record(manifest.terminal_id),
        &crate::model::ExitRecord {
            runtime_epoch: record.runtime_epoch,
            code: Some(0),
            signal: None,
            reason: crate::model::ExitReason::Natural,
            observed_at_ms: now_ms(),
            last_output_sequence: 0,
        },
    )
    .unwrap();

    let result = host.reconcile().await;
    assert_eq!(result.unreachable, 1);
    assert_eq!(result.needs_recreate, 1);
    let stored = host
        .catalog
        .lock()
        .unwrap()
        .get(manifest.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.recovery_state, RecoveryState::NeedsRecreate);
}

#[tokio::test]
async fn same_boot_worker_crash_without_strict_evidence_converges_once() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(directory.path().join("missing-runtime"));
    let host = Host::open(config).unwrap();
    let manifest = fixture_manifest(directory.path());
    let mut record = host_runtime_record(
        &host,
        manifest.terminal_id,
        directory.path().join("dead-worker.sock"),
    );
    record.worker_pid = u32::MAX;
    record.worker_start_time = 1;
    record.child_pid = u32::MAX;
    record.child_start_time = 1;
    persist_host_session(&host, &manifest, &record);

    let first = host.reconcile().await;
    assert_eq!(first.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(first.needs_recreate, usize::default());
    let second = host.reconcile().await;
    assert_eq!(second.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(second.needs_recreate, usize::default());
    let stored = host
        .catalog
        .lock()
        .unwrap()
        .get(manifest.terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(stored.recovery_state, RecoveryState::Uncertain);
}
