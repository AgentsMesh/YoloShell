use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn survivor_resume_rejects_authenticated_runtime_record_race() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (manifest, transaction, runtime, mut child) =
        survivor_resume_fault_fixture(&host, directory.path());
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    let authenticated = runtime.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let mut raced = authenticated.clone();
        raced.started_at_ms += SURVIVOR_RUNTIME_RACE_OFFSET;
        atomic_write_json(&runtime_path, &raced).unwrap();
        write_frame(
            &mut stream,
            &ResponseEnvelope::success(&request, &test_worker_hello(&authenticated)).unwrap(),
        )
        .await
        .unwrap();
    });

    let error = resume_survivor_explicit(&host, manifest.terminal_id)
        .await
        .unwrap_err()
        .into_explicit_error();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_survivor_authority_retained(&host, manifest.terminal_id);
}

#[tokio::test]
async fn survivor_resume_propagates_catalog_persistence_failure_before_parking() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let (manifest, transaction, runtime, mut child) =
        survivor_resume_fault_fixture(&host, directory.path());
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);
    *lock_recover(&host.catalog_test_probes.upsert_failure) = true;
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_resume_auth_once(listener, runtime));

    assert!(
        resume_survivor_explicit(&host, manifest.terminal_id)
            .await
            .is_err()
    );
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert_survivor_authority_retained(&host, manifest.terminal_id);
    let durable = host.load_durable_manifest(manifest.terminal_id).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert!(host.resume_transaction_needs_reconcile(&durable));
}

fn survivor_resume_fault_fixture(
    host: &Host,
    directory: &Path,
) -> (
    TerminalManifest,
    ResumeTransaction,
    RuntimeRecord,
    std::process::Child,
) {
    let mut manifest = fixture_manifest(directory);
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(host, &manifest);
    manifest.command = launch.command.clone();
    manifest.args = launch.args.clone();
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let child = spawn_resume_test_child();
    let runtime = planned_resume_runtime(
        host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Survivor,
    );
    (manifest, transaction, runtime, child)
}

pub(super) fn assert_survivor_authority_retained(host: &Host, terminal_id: Uuid) {
    assert!(host.paths.resume_transaction(terminal_id).exists());
    assert!(host.paths.worker_launch(terminal_id).exists());
    assert!(host.paths.terminate_tombstone(terminal_id).exists());
}
