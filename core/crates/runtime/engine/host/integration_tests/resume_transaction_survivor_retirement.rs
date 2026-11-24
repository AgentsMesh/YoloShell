use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn dead_survivor_rearms_one_durable_plan_and_consumes_rolled_tombstone() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let transaction = recreate_survivor_transaction(&host, &manifest, Uuid::new_v4());
    let socket = transaction.launch.socket_path.clone();
    let mut worker =
        ArchiveWorkerProcessFixture::spawn(socket, ArchiveWorkerFixtureMode::ReadyHold);
    let mut runtime = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        BootstrapDelivery::Survivor,
    );
    worker.configure(&mut runtime);
    host.secondary_custody.authorize_rpc_test(&runtime);
    let view = park_recreate_survivor(&host, &manifest, &transaction, &runtime).await;
    assert_eq!(view.manifest.recovery_state, RecoveryState::Uncertain);
    assert!(!host.resume_transaction_needs_reconcile(&view.manifest));

    worker.terminate_worker();
    assert!(process_identity_is_gone(
        runtime.worker_pid,
        runtime.worker_start_time
    ));
    assert!(!process_identity_is_gone(
        runtime.child_pid,
        runtime.child_start_time
    ));
    let durable = host.load_durable_manifest(manifest.terminal_id).unwrap();
    assert!(!host.resume_transaction_needs_reconcile(&durable));
    let parked = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(parked.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );

    worker.terminate_child();
    #[cfg(target_os = "linux")]
    crate::host::linux_retirement_evidence::publish(&host, &runtime);
    let durable = host.load_durable_manifest(manifest.terminal_id).unwrap();
    assert!(host.resume_transaction_needs_reconcile(&durable));
    let result = host.reconcile_manifest(durable).await;
    assert_eq!(result.unreachable, SURVIVOR_REARM_FAILED_RECONCILE_COUNT);
    let rearmed: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    assert_ne!(rearmed.transaction_digest, transaction.transaction_digest);
    assert_eq!(rearmed.previous_runtime_epoch, runtime.runtime_epoch);
    assert_eq!(rearmed.intent, transaction.intent);
    assert_eq!(rearmed.target, transaction.target);
    let binding_marker: ResumeBindingClearMarker =
        read_json(&host.paths.resume_binding_clear(manifest.terminal_id)).unwrap();
    binding_marker.validate_for(&rearmed).unwrap();
    assert_eq!(binding_marker.phase, ResumeBindingClearPhase::Cleared);
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(manifest.terminal_id)).unwrap(),
        rearmed.launch
    );
    let rolled = read_resume_tombstone(&host, manifest.terminal_id);
    assert_eq!(rolled.runtime_epoch, runtime.runtime_epoch);

    let mut resumed_child = spawn_resume_test_child();
    let resumed_runtime = planned_resume_runtime(
        &host,
        &rearmed,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        resumed_child.id(),
        process::process_start_time(resumed_child.id()).unwrap(),
        BootstrapDelivery::NotRequired,
    );
    host.write_namespace_json(
        &host.paths.runtime_record(manifest.terminal_id),
        &resumed_runtime,
    )
    .unwrap();
    host.secondary_custody.authorize_rpc_test(&resumed_runtime);
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &resumed_runtime.socket_path).await;
    let server = tokio::spawn(serve_ready_resume(listener, resumed_runtime));
    let completed = host
        .resume_session_locked(manifest.terminal_id, ResumeSelection::ExplicitRecreate)
        .await
        .unwrap();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut resumed_child);

    assert_eq!(completed.manifest.recovery_state, RecoveryState::Recreated);
    assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    assert!(!host.paths.terminate_tombstone(manifest.terminal_id).exists());
    assert!(
        !host
            .paths
            .resume_binding_clear(manifest.terminal_id)
            .exists()
    );
}
