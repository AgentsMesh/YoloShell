use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn durable_resume_ready_runtime_consumes_tombstone_then_commits() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESUME_TEST_BOOT_ID.into();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let previous_runtime_epoch = Uuid::new_v4();
    let transaction = shell_resume_transaction(previous_runtime_epoch, &launch);
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
    host.write_namespace_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &crate::model::TerminationTombstone::user_request(
            manifest.terminal_id,
            previous_runtime_epoch,
            false,
        ),
    )
    .unwrap();
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_ready_resume(listener, runtime));

    let view = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert_eq!(view.manifest.recovery_state, RecoveryState::Recreated);
    let observed = view.runtime.as_ref().unwrap();
    assert_eq!(observed.status, TerminalRuntimeStatus::Running);
    assert_eq!(observed.capabilities, Some(Vec::new()));
    assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
    assert!(!host.paths.terminate_tombstone(manifest.terminal_id).exists());
    let durable: TerminalManifest = read_json(&host.paths.manifest(manifest.terminal_id)).unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Recreated);
}

#[tokio::test]
async fn durable_resume_final_manifest_is_idempotently_completed() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESUME_TEST_BOOT_ID.into();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Recreated;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut runtime = planned_resume_runtime(
        &host,
        &transaction,
        RESUME_TEST_DEAD_WORKER_PID,
        RESUME_TEST_PROCESS_START_TIME,
        RESUME_TEST_DEAD_CHILD_PID,
        RESUME_TEST_PROCESS_START_TIME,
        BootstrapDelivery::NotRequired,
    );
    runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();

    let view = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap();
    assert_eq!(view.manifest.recovery_state, RecoveryState::Recreated);
    assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
    assert_eq!(
        lock_recover(&host.catalog)
            .get(manifest.terminal_id)
            .unwrap()
            .unwrap()
            .recovery_state,
        RecoveryState::Recreated
    );
}

#[tokio::test]
async fn durable_resume_mismatched_tombstone_blocks_completion() {
    let directory = tempdir().unwrap();
    let mut host = Host::open(HostConfig::new(directory.path())).unwrap();
    host.boot_id = RESUME_TEST_BOOT_ID.into();
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
    host.write_namespace_json(
        &host.paths.terminate_tombstone(manifest.terminal_id),
        &crate::model::TerminationTombstone::user_request(
            manifest.terminal_id,
            Uuid::new_v4(),
            false,
        ),
    )
    .unwrap();
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_resume_auth_once(listener, runtime));

    let error = host
        .resume_session_locked(
            manifest.terminal_id,
            ResumeSelection::Preserve(ResumeIntent::Explicit),
        )
        .await
        .unwrap_err()
        .into_explicit_error();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert_eq!(error.code, crate::protocol::error_code::CONFLICT);
    assert!(host.paths.resume_transaction(manifest.terminal_id).exists());
    assert!(host.paths.terminate_tombstone(manifest.terminal_id).exists());
    assert_eq!(
        read_json::<TerminalManifest>(&host.paths.manifest(manifest.terminal_id))
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
}

#[test]
fn resume_completion_preserves_a_new_tombstone_at_remove_boundary() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let manifest = fixture_manifest(directory.path());
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
    let path = host.paths.terminate_tombstone(manifest.terminal_id);
    let previous = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        transaction.previous_runtime_epoch,
        false,
    );
    let replacement = crate::model::TerminationTombstone::user_request(
        manifest.terminal_id,
        transaction.planned_runtime_epoch,
        true,
    );
    host.write_namespace_json(&path, &previous).unwrap();

    let result = host.consume_resume_tombstone_with_boundary(&transaction, || {
        host.write_namespace_json(&path, &replacement)
    });

    assert!(result.is_ok());
    assert_eq!(
        read_json::<crate::model::TerminationTombstone>(&path).unwrap(),
        replacement
    );
    assert_eq!(
        host.automatic_restore_decision(&manifest),
        AutomaticRestore::Ineligible
    );
}
