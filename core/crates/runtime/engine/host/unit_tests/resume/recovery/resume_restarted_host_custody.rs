use crate::host::tests::*;
use super::*;

#[tokio::test]
async fn pending_resume_restarted_host_adopts_custody_before_controller_authentication() {
    use crate::pty_custody::{CustodyDurableRecord, CustodyRuntimeIdentity, CustodyUnixStream};

    for delivery in [BootstrapDelivery::NotRequired, BootstrapDelivery::Survivor] {
        let directory = tempdir().unwrap();
        let first = Host::open(HostConfig::new(directory.path())).unwrap();
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = RecoveryState::Restoring;
        let launch = shell_resume_launch(&first, &manifest);
        let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
        let mut child = spawn_resume_test_child();
        let runtime = planned_resume_runtime(
            &first,
            &transaction,
            std::process::id(),
            process::process_start_time(std::process::id()).unwrap(),
            child.id(),
            process::process_start_time(child.id()).unwrap(),
            delivery,
        );
        persist_host_session(&first, &manifest, &runtime);
        first.publish_resume_transaction(&transaction).unwrap();
        first.publish_worker_launch(&launch).unwrap();
        let custody = CustodyDurableRecord::pending(CustodyRuntimeIdentity::from_record(&runtime));
        first
            .write_namespace_json(&first.paths.pty_custody(runtime.terminal_id), &custody)
            .unwrap();
        drop(first);

        let host = Host::open(HostConfig::new(directory.path())).unwrap();
        assert!(host.controller_generation > runtime.controller_generation);
        assert!(!host.rpc_custody_ready(&runtime));
        let (listener, publication) =
            publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
        let expected = runtime.clone();
        let generation = host.controller_generation;
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let opening: RequestEnvelope = read_frame(&mut stream).await.unwrap();
            let request = CustodyUnixStream::decode_opening(&opening).unwrap();
            assert_eq!(
                request.runtime,
                CustodyRuntimeIdentity::from_record(&expected)
            );
            assert_eq!(request.controller_generation, generation);
            assert_eq!(request.auth_token(), expected.auth_token);
            // Disconnect before Prepared: recovery must retain evidence and never
            // authenticate, publish completion, or signal the live shell.
        });
        let result = host
            .continue_planned_runtime(manifest, transaction.clone(), runtime.clone())
            .await;
        tokio::time::timeout(crate::session_timing::WORKER_AUTH_TIMEOUT, server)
            .await
            .unwrap()
            .unwrap();
        drop(publication);
        assert!(result.is_err());
        assert!(!host.rpc_custody_ready(&runtime));
        assert!(process::is_process_alive(child.id()));
        let current: RuntimeRecord = host
            .read_namespace_json(&host.paths.runtime_record(runtime.terminal_id))
            .unwrap();
        assert_eq!(current, runtime);
        assert_eq!(
            host.load_resume_transaction(runtime.terminal_id).unwrap(),
            Some(transaction)
        );
        assert!(host.paths.worker_launch(runtime.terminal_id).exists());
        stop_resume_test_child(&mut child);
    }
}
