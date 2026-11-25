use crate::host::tests::*;
#[tokio::test]
async fn resume_replayed_launch_completes_after_worker_readiness() {
    let directory = tempdir().unwrap();
    let mut config = HostConfig::new(directory.path());
    config.runtime_executable = Some(PathBuf::from(RESUME_IMMEDIATE_EXIT_RUNTIME));
    let host = Host::open(config).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::NotRequired);
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    host.publish_worker_launch(&launch).unwrap();
    let (listener, publication) =
        publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
    let server = tokio::spawn(serve_replayed_resume_ready(listener, runtime));

    let view = host
        .replay_resume_launch(manifest, transaction, launch)
        .await
        .unwrap();

    server.await.unwrap();
    drop(publication);
    assert_eq!(view.manifest.recovery_state, RecoveryState::Recreated);
    assert_eq!(view.runtime.unwrap().status, TerminalRuntimeStatus::Running);
    assert!(
        !host
            .paths
            .resume_transaction(view.manifest.terminal_id)
            .exists()
    );
    assert!(!host.paths.worker_launch(view.manifest.terminal_id).exists());
}
