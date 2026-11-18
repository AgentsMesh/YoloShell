use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn previous_boot_provider_rearm_preserves_plan_with_missing_launch() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let (_, _, transaction) = provider_resume_fixture(&host, &manifest);
    let mut child = spawn_resume_test_child();
    let mut runtime = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Survivor,
    );
    runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);
    let mut rolled = read_resume_tombstone(&host, manifest.terminal_id);
    rolled.runtime_epoch = runtime.runtime_epoch;
    rolled.force = true;
    host.write_namespace_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &rolled,
    )
    .unwrap();
    host.namespace
        .remove_file(&host.paths.worker_launch(manifest.terminal_id))
        .unwrap();

    let error = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    let rearmed: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    assert_eq!(rearmed.previous_runtime_epoch, runtime.runtime_epoch);
    assert_eq!(rearmed.intent, transaction.intent);
    assert_eq!(rearmed.target, transaction.target);
    assert!(rearmed.launch.initial_input.is_some());
    assert_eq!(read_resume_tombstone(&host, manifest.terminal_id), rolled);
    assert!(!process_identity_is_gone(
        runtime.child_pid,
        runtime.child_start_time
    ));
    stop_resume_test_child(&mut child);
}

#[tokio::test]
async fn replaced_transaction_without_launch_replays_the_same_plan() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Uncertain;
    let mut previous = dead_resume_runtime(&host, &manifest);
    previous.bootstrap_delivery = BootstrapDelivery::Survivor;
    let transaction = recreate_survivor_transaction(&host, &manifest, previous.runtime_epoch);
    persist_host_session(&host, &manifest, &previous);
    #[cfg(target_os = "linux")]
    crate::host::linux_retirement_evidence::publish(&host, &previous);
    host.publish_resume_transaction(&transaction).unwrap();
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    assert!(host.resume_transaction_needs_reconcile(&manifest));

    let result = host.reconcile_manifest(manifest).await;
    assert_eq!(result.unreachable, SURVIVOR_REARM_FAILED_RECONCILE_COUNT);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(transaction.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(transaction.terminal_id)).unwrap(),
        transaction.launch
    );
}
