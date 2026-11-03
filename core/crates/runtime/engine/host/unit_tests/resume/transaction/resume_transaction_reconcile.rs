use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn durable_resume_sidecar_precedes_needs_recreate_catalog_state() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::NeedsRecreate;
    let previous = persist_dead_resume_session(&host, &manifest);
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = ResumeTransaction::new(
        previous.runtime_epoch,
        ResumeIntent::Automatic,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Preserve,
        },
        &launch,
    )
    .unwrap();
    host.publish_resume_transaction(&transaction).unwrap();
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());

    let result = host.reconcile_manifest(manifest.clone()).await;
    assert_eq!(result.unreachable, 1);
    assert_eq!(result.needs_recreate, 0);
    assert_eq!(
        read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(manifest.terminal_id)).unwrap(),
        launch
    );
    assert_eq!(
        read_json::<TerminalManifest>(&host.paths.manifest(manifest.terminal_id))
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
    let parked_manifest = host.load_durable_manifest(manifest.terminal_id).unwrap();
    let transaction_bytes =
        std::fs::read(host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    let launch_bytes = std::fs::read(host.paths.worker_launch(manifest.terminal_id)).unwrap();
    assert!(!host.resume_transaction_needs_reconcile(&parked_manifest));
    let parked = host.reconcile_manifest(parked_manifest.clone()).await;
    assert_eq!(parked.unreachable, 1);
    assert_eq!(
        std::fs::read(host.paths.resume_transaction(manifest.terminal_id)).unwrap(),
        transaction_bytes
    );
    assert_eq!(
        std::fs::read(host.paths.worker_launch(manifest.terminal_id)).unwrap(),
        launch_bytes
    );
    let explicit_error = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    assert_eq!(
        explicit_error.code,
        crate::protocol::error_code::WORKER_START_FAILED
    );
    assert!(!host.resume_transaction_needs_reconcile(&parked_manifest));
    drop(host);
    let reopened = unspawnable_resume_host(&directory);
    assert!(reopened.resume_transaction_needs_reconcile(&parked_manifest));
    let replayed = reopened.reconcile_manifest(parked_manifest.clone()).await;
    assert_eq!(replayed.unreachable, 1);
    assert!(!reopened.resume_transaction_needs_reconcile(&parked_manifest));
    assert_eq!(
        std::fs::read(reopened.paths.resume_transaction(manifest.terminal_id)).unwrap(),
        transaction_bytes
    );
    assert_eq!(
        std::fs::read(reopened.paths.worker_launch(manifest.terminal_id)).unwrap(),
        launch_bytes
    );
    let replayed_manifest = reopened.load_durable_manifest(manifest.terminal_id).unwrap();
    let parked_again = reopened.reconcile_manifest(replayed_manifest).await;
    assert_eq!(parked_again.unreachable, 1);
    assert_eq!(
        std::fs::read(reopened.paths.resume_transaction(manifest.terminal_id)).unwrap(),
        transaction_bytes
    );
    assert_eq!(
        std::fs::read(reopened.paths.worker_launch(manifest.terminal_id)).unwrap(),
        launch_bytes
    );
}

#[tokio::test]
async fn durable_resume_missing_sidecar_does_not_override_needs_recreate() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::NeedsRecreate;
    persist_dead_resume_session(&host, &manifest);

    assert!(!host.resume_transaction_needs_reconcile(&manifest));
}

#[test]
fn durable_resume_transaction_raises_catalog_generation_floor_before_launch() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let previous = persist_dead_resume_session(&host, &manifest);
    let mut launch = shell_resume_launch(&host, &manifest);
    launch.controller_generation = host.controller_generation + RESUME_TEST_GENERATION_OFFSET;
    let transaction = shell_resume_transaction(previous.runtime_epoch, &launch);
    host.publish_resume_transaction(&transaction).unwrap();

    let floor =
        catalog_rebuild::durable_controller_generation(&host.paths, &host.namespace).unwrap();
    assert_eq!(floor, launch.controller_generation);
    assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
}

#[test]
fn late_planned_runtime_reopens_an_evidence_parked_resume() {
    let directory = tempdir().unwrap();
    let first = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let previous = persist_dead_resume_session(&first, &manifest);
    let launch = shell_resume_launch(&first, &manifest);
    let transaction = ResumeTransaction::new(
        previous.runtime_epoch,
        ResumeIntent::Automatic,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Preserve,
        },
        &launch,
    )
    .unwrap();
    first.publish_resume_transaction(&transaction).unwrap();
    drop(first);

    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    assert!(transaction.launch.controller_generation < host.controller_generation);
    lock_recover(&host.parked_resume_retries).insert(
        resume_retry_key(&transaction),
        ResumeRetryDisposition::EvidenceRequired,
    );
    let planned = planned_resume_runtime(
        &host,
        &transaction,
        previous.worker_pid,
        previous.worker_start_time,
        previous.child_pid,
        previous.child_start_time,
        BootstrapDelivery::NotRequired,
    );
    host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &planned)
        .unwrap();

    assert!(host.resume_transaction_needs_reconcile(&manifest));
    assert!(
        host.authorize_parked_resume_retry(
            &transaction,
            ResumeSelection::Preserve(ResumeIntent::Automatic),
        )
        .is_ok()
    );
}
