use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn planned_and_committed_resume_reject_incompatible_durable_evidence() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESUME_TEST_BOOT_ID.into();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let runtime = planned_resume_runtime(
        &host,
        &transaction,
        RESUME_TEST_DEAD_WORKER_PID,
        RESUME_TEST_PROCESS_START_TIME,
        RESUME_TEST_DEAD_CHILD_PID,
        RESUME_TEST_PROCESS_START_TIME,
        BootstrapDelivery::NotRequired,
    );
    persist_host_session(&host, &manifest, &runtime);

    assert!(
        host.finish_planned_resume(manifest.clone(), &transaction)
            .await
            .is_err()
    );

    manifest.recovery_state = RecoveryState::Resumed;
    host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &manifest)
        .unwrap();
    assert!(
        host.finish_committed_resume(manifest, &transaction)
            .await
            .is_err()
    );
}

#[test]
fn resume_tombstone_noop_paths_preserve_new_or_absent_intent() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let previous_epoch = Uuid::new_v4();
    let explicit = shell_resume_transaction(previous_epoch, &launch);
    host.paths.ensure_session(manifest.terminal_id).unwrap();

    host.consume_resume_tombstone(&explicit).unwrap();

    let path = host.paths.terminate_tombstone(manifest.terminal_id);
    let tombstone = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        previous_epoch,
        false,
    );
    host.write_namespace_json(&path, &tombstone).unwrap();
    let automatic = ResumeTransaction::new(
        previous_epoch,
        ResumeIntent::Automatic,
        ResumeTarget::Shell {
            binding: ResumeBindingDisposition::Preserve,
        },
        &launch,
    )
    .unwrap();
    host.consume_resume_tombstone(&automatic).unwrap();
    assert_eq!(
        read_json::<crate::model::TerminationTombstone>(&path).unwrap(),
        tombstone
    );

    host.consume_resume_tombstone_with_boundary(&explicit, || host.namespace.remove_file(&path))
        .unwrap();
    assert!(!path.exists());
}

#[tokio::test]
async fn continue_resume_rejects_manifest_and_runtime_evidence_gaps() {
    assert_continue_failure(ContinueFailure::InvalidManifest).await;
    assert_continue_failure(ContinueFailure::MissingRuntime).await;
    assert_continue_failure(ContinueFailure::UnexpectedRuntime).await;
}

#[tokio::test]
async fn planned_resume_rejects_terminal_or_dead_pending_delivery() {
    for delivery in [
        BootstrapDelivery::Pending,
        BootstrapDelivery::Failed,
        BootstrapDelivery::Unknown,
    ] {
        let directory = tempdir().unwrap();
        let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
        host.boot_id = RESUME_TEST_BOOT_ID.into();
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = RecoveryState::Restoring;
        let launch = shell_resume_launch(&host, &manifest);
        let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
        let runtime = planned_resume_runtime(
            &host,
            &transaction,
            RESUME_TEST_DEAD_WORKER_PID,
            RESUME_TEST_PROCESS_START_TIME,
            RESUME_TEST_DEAD_CHILD_PID,
            RESUME_TEST_PROCESS_START_TIME,
            delivery,
        );
        persist_host_session(&host, &manifest, &runtime);

        assert!(
            host.continue_planned_runtime(manifest, transaction, runtime)
                .await
                .is_err()
        );
    }
}

enum ContinueFailure {
    InvalidManifest,
    MissingRuntime,
    UnexpectedRuntime,
}

async fn assert_continue_failure(failure: ContinueFailure) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut runtime = dead_resume_runtime(&host, &manifest);
    match failure {
        ContinueFailure::InvalidManifest => manifest.recovery_state = RecoveryState::Resumed,
        ContinueFailure::MissingRuntime => {}
        ContinueFailure::UnexpectedRuntime => runtime.runtime_epoch = Uuid::new_v4(),
    }
    persist_host_session(&host, &manifest, &runtime);
    if matches!(failure, ContinueFailure::MissingRuntime) {
        host.namespace
            .remove_file(&host.paths.runtime_record(manifest.terminal_id))
            .unwrap();
    }

    assert!(
        host.continue_resume_transaction(manifest, transaction)
            .await
            .is_err()
    );
}
