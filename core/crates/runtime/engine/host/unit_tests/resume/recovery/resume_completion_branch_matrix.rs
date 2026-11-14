use crate::host::tests::*;
use super::*;

const RESUME_COMPLETION_RACE_OFFSET: i64 = 1;

#[tokio::test]
async fn resume_completion_rejects_a_persisted_runtime_changed_after_authentication() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut child = spawn_resume_test_child();
    let runtime = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::NotRequired,
    );
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let runtime_path = host.paths.runtime_record(manifest.terminal_id);
    let authenticated = runtime.clone();
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request: RequestEnvelope = read_frame(&mut stream).await.unwrap();
        let mut changed = authenticated.clone();
        changed.started_at_ms = changed
            .started_at_ms
            .saturating_add(RESUME_COMPLETION_RACE_OFFSET);
        atomic_write_json(&runtime_path, &changed).unwrap();
        let response =
            ResponseEnvelope::success(&request, &test_worker_hello(&authenticated)).unwrap();
        write_frame(&mut stream, &response).await.unwrap();
    });

    assert!(
        host.finish_planned_resume(manifest, &transaction)
            .await
            .is_err()
    );
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert!(
        host.paths
            .resume_transaction(transaction.terminal_id)
            .exists()
    );
}

#[test]
fn resume_completion_removes_exact_launch_and_settles_tombstone_races() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    host.paths.ensure_session(manifest.terminal_id).unwrap();

    host.publish_worker_launch(&launch).unwrap();
    host.remove_completed_resume_launch(&manifest, &transaction)
        .unwrap();
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());

    let path = host.paths.terminate_tombstone(manifest.terminal_id);
    let previous = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        transaction.previous_runtime_epoch,
        false,
    );
    let mut changed_previous = previous.clone();
    changed_previous.force = true;
    host.write_namespace_json(&path, &previous).unwrap();
    host.consume_resume_tombstone_with_boundary(&transaction, || {
        atomic_write_json(&path, &changed_previous)
    })
    .unwrap();
    assert!(!path.exists());

    let planned = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        transaction.planned_runtime_epoch,
        true,
    );
    host.write_namespace_json(&path, &previous).unwrap();
    let boundary_path = path.clone();
    let boundary_planned = planned.clone();
    *lock_recover(&host.catalog_test_probes.tombstone_consume) = Some(Box::new(move || {
        atomic_write_json(&boundary_path, &boundary_planned)
    }));
    host.consume_resume_tombstone(&transaction).unwrap();
    assert_eq!(
        read_json::<crate::model::TerminationTombstone>(&path).unwrap(),
        planned
    );
}

#[tokio::test]
async fn resume_finish_or_retain_routes_survivor_without_consuming_authority() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut child = spawn_resume_test_child();
    let runtime = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Survivor,
    );
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_ready_resume(listener, runtime.clone()));

    let view = host
        .finish_or_retain_planned_resume(manifest, &transaction)
        .await
        .unwrap();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert_eq!(view.manifest.recovery_state, RecoveryState::Uncertain);
    assert!(
        host.paths
            .resume_transaction(transaction.terminal_id)
            .exists()
    );
    assert_eq!(
        view.runtime.unwrap().bootstrap_delivery,
        BootstrapDelivery::Survivor
    );
}
