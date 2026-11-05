use crate::host::tests::*;
use super::*;

const RESUME_CONTINUATION_COMMAND_MISMATCH: &str = "resume-command-mismatch";
const RESUME_CONTINUATION_ARGUMENT_MISMATCH: &str = "resume-argument-mismatch";
pub(super) const RESUME_IMMEDIATE_EXIT_RUNTIME: &str = crate::test_executables::TRUE;

enum RestoringMutation {
    None,
    State,
    Command,
    Arguments,
}

#[tokio::test]
async fn resume_restoring_manifest_change_predicate_covers_each_operand() {
    for mutation in [
        RestoringMutation::None,
        RestoringMutation::State,
        RestoringMutation::Command,
        RestoringMutation::Arguments,
    ] {
        let directory = tempdir().unwrap();
        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        let mut manifest = fixture_manifest(directory.path());
        let command = shell_command(Path::new(&manifest.cwd));
        manifest.command = command.executable.to_string_lossy().into_owned();
        manifest.args = command.args;
        manifest.recovery_state = RecoveryState::Restoring;
        match mutation {
            RestoringMutation::None => {}
            RestoringMutation::State => manifest.recovery_state = RecoveryState::Uncertain,
            RestoringMutation::Command => manifest
                .command
                .push_str(RESUME_CONTINUATION_COMMAND_MISMATCH),
            RestoringMutation::Arguments => manifest
                .args
                .push(RESUME_CONTINUATION_ARGUMENT_MISMATCH.into()),
        }
        let runtime = dead_resume_runtime(&host, &manifest);
        persist_host_session(&host, &manifest, &runtime);

        host.ensure_restoring_manifest(&mut manifest).await.unwrap();

        let expected = shell_command(Path::new(&manifest.cwd));
        assert_eq!(manifest.recovery_state, RecoveryState::Restoring);
        assert_eq!(manifest.command, expected.executable.to_string_lossy());
        assert_eq!(manifest.args, expected.args);
    }
}

async fn serve_resume_ready_after_pending(listener: UnixListener, runtime: RuntimeRecord) {
    let (pending_stream, _) = listener.accept().await.unwrap();
    let mut pending_stream = UnixStream::from(pending_stream);
    let mut pending_stream = accept_worker_auth(pending_stream, &runtime).await;
    serve_ready_resume(listener, runtime).await;
}

#[tokio::test]
async fn resume_pending_runtime_covers_ready_and_failed_continuations() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut child = spawn_resume_test_child();
    let pending = planned_resume_runtime(
        &host,
        &transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        child.id(),
        process::process_start_time(child.id()).unwrap(),
        BootstrapDelivery::Pending,
    );
    let mut ready = pending.clone();
    ready.bootstrap_delivery = BootstrapDelivery::NotRequired;
    persist_host_session(&host, &manifest, &ready);
    host.publish_resume_transaction(&transaction).unwrap();
    host.publish_worker_launch(&launch).unwrap();
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &ready.socket_path).await;
    let server = tokio::spawn(serve_resume_ready_after_pending(listener, ready));

    let view = host
        .continue_planned_runtime(manifest, transaction, pending)
        .await
        .unwrap();
    server.await.unwrap();
    drop(publication);
    stop_resume_test_child(&mut child);
    assert_eq!(view.manifest.recovery_state, RecoveryState::Recreated);

    let failed_directory = tempdir().unwrap();
    let failed_host = Host::open(HostConfig::new(failed_directory.path())).unwrap();
    let mut failed_manifest = fixture_manifest(failed_directory.path());
    failed_manifest.recovery_state = RecoveryState::Restoring;
    let failed_launch = shell_resume_launch(&failed_host, &failed_manifest);
    let failed_transaction = shell_resume_transaction(Uuid::new_v4(), &failed_launch);
    let mut failed_child = spawn_resume_test_child();
    let failed_pending = planned_resume_runtime(
        &failed_host,
        &failed_transaction,
        std::process::id(),
        process::process_start_time(std::process::id()).unwrap(),
        failed_child.id(),
        process::process_start_time(failed_child.id()).unwrap(),
        BootstrapDelivery::Pending,
    );
    let mut failed = failed_pending.clone();
    failed.bootstrap_delivery = BootstrapDelivery::Failed;
    persist_host_session(&failed_host, &failed_manifest, &failed);

    assert!(
        failed_host
            .continue_planned_runtime(failed_manifest, failed_transaction, failed_pending)
            .await
            .is_err()
    );
    stop_resume_test_child(&mut failed_child);
}

#[tokio::test]
async fn zero_deadline_resume_replay_stays_pre_spawn_and_retryable() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(PathBuf::from(RESUME_IMMEDIATE_EXIT_RUNTIME));
    let mut host = Host::open(config).unwrap();
    host.worker_ready_timeout = Duration::ZERO;
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let previous = dead_resume_runtime(&host, &manifest);
    persist_host_session(&host, &manifest, &previous);
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(previous.runtime_epoch, &launch);
    host.publish_resume_transaction(&transaction).unwrap();
    host.publish_worker_launch(&launch).unwrap();
    let terminal_id = manifest.terminal_id;

    let error = host
        .replay_resume_launch(manifest, transaction, launch)
        .await
        .unwrap_err()
        .into_explicit_error();

    assert_eq!(error.code, crate::protocol::error_code::WORKER_START_FAILED);
    assert_eq!(
        host.load_durable_manifest(terminal_id)
            .unwrap()
            .recovery_state,
        RecoveryState::Uncertain
    );
    assert!(host.paths.resume_transaction(terminal_id).exists());
    assert!(host.paths.worker_launch(terminal_id).exists());
    assert!(!host.paths.worker_startup_checkpoint(terminal_id).exists());
}
