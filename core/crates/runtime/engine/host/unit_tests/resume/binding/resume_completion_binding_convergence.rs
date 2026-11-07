use crate::host::tests::*;
const RESUME_TOMBSTONE_SAVED_EXTENSION: &str = "resume-saved";

#[tokio::test]
async fn resume_planned_completion_rejects_an_unexpected_binding_marker() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::NotRequired);
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    write_unexpected_resume_binding_marker(&host, &manifest);
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_resume_auth_once(listener, runtime));

    let result = host.finish_planned_resume(manifest, &transaction).await;

    server.await.unwrap();
    drop(publication);
    assert_resume_failure_retains(&host, &transaction, result);
}

#[tokio::test]
async fn resume_planned_completion_revalidates_binding_marker_after_manifest_commit() {
    let directory = tempdir().unwrap();
    let host = Arc::new(Host::open(HostConfig::new(directory.path())).unwrap());
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let transaction = binding_clear_transaction(&host, &manifest, None);
    let runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::NotRequired);
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    host.publish_worker_launch(&transaction.launch).unwrap();
    host.stage_resume_binding_disposition(&transaction).unwrap();
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_resume_auth_once(listener, runtime));
    let catalog = Arc::clone(&host.catalog);
    let (locked, locked_ready) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let blocker = std::thread::spawn(move || {
        let _guard = lock_recover(&catalog);
        locked.send(()).unwrap();
        released.recv_timeout(HOST_BLOCKING_TEST_TIMEOUT).unwrap();
    });
    locked_ready
        .recv_timeout(HOST_BLOCKING_TEST_TIMEOUT)
        .unwrap();
    let (operations, mut observed) = tokio::sync::mpsc::unbounded_channel();
    *lock_recover(&host.catalog_test_probes.operations) = Some(operations);
    let task_host = Arc::clone(&host);
    let task_transaction = transaction.clone();
    let task = tokio::spawn(async move {
        task_host
            .finish_planned_resume(manifest, &task_transaction)
            .await
    });

    tokio::time::timeout(HOST_BLOCKING_TEST_TIMEOUT, observed.recv())
        .await
        .unwrap()
        .unwrap();
    let pending = binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
    host.write_namespace_json(
        &host.paths.resume_binding_clear(transaction.terminal_id),
        &pending,
    )
    .unwrap();
    release.send(()).unwrap();
    blocker.join().unwrap();
    let result = task.await.unwrap();

    server.await.unwrap();
    drop(publication);
    assert_resume_failure_retains(&host, &transaction, result);
    assert_eq!(
        read_json::<ResumeBindingClearMarker>(
            &host.paths.resume_binding_clear(transaction.terminal_id)
        )
        .unwrap()
        .phase,
        ResumeBindingClearPhase::Pending
    );
}

#[test]
fn resume_tombstone_retry_failure_and_foreign_epoch_fail_closed() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let transaction =
        shell_resume_transaction(Uuid::new_v4(), &shell_resume_launch(&host, &manifest));
    host.paths.ensure_session(manifest.terminal_id).unwrap();
    let path = host.paths.terminate_tombstone(manifest.terminal_id);
    let foreign = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        Uuid::new_v4(),
        false,
    );
    host.write_namespace_json(&path, &foreign).unwrap();
    assert!(host.resume_tombstone(&transaction).is_err());
    host.namespace.remove_file(&path).unwrap();
    let previous = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        transaction.previous_runtime_epoch,
        false,
    );
    host.write_namespace_json(&path, &previous).unwrap();
    let saved = path.with_extension(RESUME_TOMBSTONE_SAVED_EXTENSION);
    std::fs::rename(&path, &saved).unwrap();
    write_private_bytes(&path, RESUME_CONVERGENCE_CORRUPT_JSON);
    host.namespace.remove_file(&path).unwrap();
    std::fs::rename(saved, &path).unwrap();

    let result = host
        .settle_changed_resume_tombstone(&transaction, anyhow::anyhow!(RESUME_TOMBSTONE_MISMATCH));

    assert!(result.is_err());
    assert_eq!(
        read_json::<crate::model::TerminationTombstone>(&path).unwrap(),
        previous
    );
}
