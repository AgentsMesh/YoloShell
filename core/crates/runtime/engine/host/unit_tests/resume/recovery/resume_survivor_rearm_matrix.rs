use crate::host::tests::*;
use super::*;

enum ResumeRearmTarget {
    Shell,
    Provider,
}

fn resume_rearm_transaction(
    host: &Host,
    manifest: &TerminalManifest,
    target: ResumeRearmTarget,
) -> ResumeTransaction {
    match target {
        ResumeRearmTarget::Shell => recreate_survivor_transaction(host, manifest, Uuid::new_v4()),
        ResumeRearmTarget::Provider => provider_resume_fixture(host, manifest).2,
    }
}

#[tokio::test]
async fn resume_dead_survivor_rearm_replaces_shell_and_provider_plans() {
    for target in [ResumeRearmTarget::Shell, ResumeRearmTarget::Provider] {
        let directory = tempdir().unwrap();
        let host = unspawnable_resume_host(&directory);
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = RecoveryState::Restoring;
        let transaction = resume_rearm_transaction(&host, &manifest, target);
        let mut runtime = planned_resume_runtime(
            &host,
            &transaction,
            RESUME_TEST_DEAD_WORKER_PID,
            RESUME_TEST_PROCESS_START_TIME,
            RESUME_TEST_DEAD_CHILD_PID,
            RESUME_TEST_PROCESS_START_TIME,
            BootstrapDelivery::Survivor,
        );
        runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
        persist_survivor_resume(&host, &manifest, &transaction, &runtime);

        let replacement = host
            .rearm_dead_resume_terminal(&mut manifest, &transaction)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(replacement.previous_runtime_epoch, runtime.runtime_epoch);
        assert_eq!(replacement.intent, transaction.intent);
        assert_eq!(replacement.target, transaction.target);
        assert_eq!(manifest.recovery_state, RecoveryState::Uncertain);
        assert_eq!(
            read_json::<ResumeTransaction>(&host.paths.resume_transaction(manifest.terminal_id))
                .unwrap(),
            replacement
        );
        assert_eq!(
            read_json::<WorkerLaunchSpec>(&host.paths.worker_launch(manifest.terminal_id)).unwrap(),
            replacement.launch
        );
    }
}

#[tokio::test]
async fn resume_survivor_rearm_covers_absent_evidence_and_active_socket_fences() {
    let missing_directory = tempdir().unwrap();
    let missing_host = unspawnable_resume_host(&missing_directory);
    let mut missing_manifest = fixture_manifest(missing_directory.path());
    missing_manifest.recovery_state = RecoveryState::Restoring;
    let previous = persist_dead_resume_session(&missing_host, &missing_manifest);
    let missing_transaction =
        recreate_survivor_transaction(&missing_host, &missing_manifest, previous.runtime_epoch);
    missing_host
        .publish_resume_transaction(&missing_transaction)
        .unwrap();
    missing_host
        .namespace
        .remove_file(
            &missing_host
                .paths
                .runtime_record(missing_manifest.terminal_id),
        )
        .unwrap();
    assert!(
        missing_host
            .rearm_dead_resume_terminal(&mut missing_manifest, &missing_transaction)
            .await
            .unwrap()
            .is_none()
    );

    let active_directory = tempdir().unwrap();
    let active_host = unspawnable_resume_host(&active_directory);
    let (mut manifest, transaction, runtime) =
        dead_survivor_rearm_fixture(&active_host, active_directory.path());
    assert!(
        active_host
            .validate_terminal_rearm_evidence(&manifest, &transaction, &runtime)
            .is_err()
    );
    let (_listener, publication) =
        publish_test_socket(&active_host.socket_namespace, &runtime.socket_path).await;
    let error = active_host
        .rearm_dead_resume_terminal(&mut manifest, &transaction)
        .await
        .unwrap_err()
        .into_explicit_error();
    drop(publication);
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert_eq!(
        read_json::<ResumeTransaction>(&active_host.paths.resume_transaction(manifest.terminal_id))
            .unwrap(),
        transaction
    );
    assert_eq!(
        read_json::<WorkerLaunchSpec>(&active_host.paths.worker_launch(manifest.terminal_id))
            .unwrap(),
        transaction.launch
    );
}
