use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn durable_resume_without_a_valid_transaction_fails_closed() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);

    let mut missing = fixture_manifest(directory.path());
    missing.recovery_state = RecoveryState::Restoring;
    persist_dead_resume_session(&host, &missing);
    let error = host
        .resume_session(ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(missing.terminal_id),
            recreate: false,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(!host.paths.worker_launch(missing.terminal_id).exists());
    assert_eq!(
        lock_recover(&host.catalog)
            .get(missing.terminal_id)
            .unwrap()
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );

    let corrupt = fixture_manifest(directory.path());
    persist_dead_resume_session(&host, &corrupt);
    host.write_namespace_json(
        &host.paths.resume_transaction(corrupt.terminal_id),
        &serde_json::json!({}),
    )
    .unwrap();
    let error = host
        .resume_session(ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(corrupt.terminal_id),
            recreate: false,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(!host.paths.worker_launch(corrupt.terminal_id).exists());
    assert_eq!(
        lock_recover(&host.catalog)
            .get(corrupt.terminal_id)
            .unwrap()
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );

    let visible = fixture_manifest(directory.path());
    let visible_runtime = persist_dead_resume_session(&host, &visible);
    persist_archive_exit(&host, &visible_runtime, visible_runtime.runtime_epoch);
    let launch = shell_resume_launch(&host, &visible);
    host.publish_worker_launch(&launch).unwrap();
    let error = host
        .resume_session(ResumeRequest {
            selection: TerminalLifecycleSelection::unconditional(visible.terminal_id),
            recreate: false,
        })
        .await
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::RECREATE_REQUIRED);
    assert!(!host.paths.resume_transaction(visible.terminal_id).exists());
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(visible.terminal_id)).unwrap(),
        launch
    );
}

#[tokio::test]
async fn durable_resume_transaction_first_replays_one_plan() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let previous = persist_dead_resume_session(&host, &manifest);
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(previous.runtime_epoch, &launch);
    host.publish_resume_transaction(&transaction).unwrap();
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    let request = ResumeRequest {
        selection: TerminalLifecycleSelection::unconditional(manifest.terminal_id),
        recreate: false,
    };

    let first = host.resume_session(request.clone()).await.unwrap_err();
    assert_eq!(first.code, crate::protocol::error_code::WORKER_START_FAILED);
    let first_transaction: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    assert_eq!(
        first_transaction.previous_runtime_epoch,
        previous.runtime_epoch
    );
    assert_eq!(first_transaction, transaction);

    let second = host.resume_session(request).await.unwrap_err();
    assert_eq!(
        second.code,
        crate::protocol::error_code::WORKER_START_FAILED
    );
    let second_transaction: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    let persisted_launch: WorkerLaunchSpec =
        read_json(&host.paths.worker_launch(manifest.terminal_id)).unwrap();
    assert_eq!(second_transaction, first_transaction);
    assert_eq!(persisted_launch, launch);
}

#[tokio::test]
async fn durable_resume_pending_runtime_is_awaited_without_another_spawn() {
    let directory = tempdir().unwrap();
    let mut host = unspawnable_resume_host(&directory);
    host.boot_id = RESUME_TEST_BOOT_ID.into();
    host.worker_ready_timeout = Duration::ZERO;
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
        BootstrapDelivery::Pending,
    );
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    host.publish_worker_launch(&launch).unwrap();
    host.park_resume_retry(
        manifest.terminal_id,
        ResumeRetryDisposition::EvidenceRequired,
    )
    .unwrap();
    assert!(host.resume_transaction_needs_reconcile(&manifest));

    let view = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap();
    stop_resume_test_child(&mut child);
    assert_eq!(view.manifest.recovery_state, RecoveryState::Uncertain);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id))
            .unwrap()
            .bootstrap_delivery,
        BootstrapDelivery::Pending
    );
    assert!(host.paths.worker_launch(manifest.terminal_id).exists());
}

include!("../recovery/resume_recreate_selection_fence.rs");
