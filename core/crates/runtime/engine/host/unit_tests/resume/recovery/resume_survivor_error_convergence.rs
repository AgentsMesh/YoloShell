use crate::host::tests::*;
#[derive(Clone, Copy)]
enum ResumeSurvivorFault {
    RuntimeRead,
    RuntimeValidation,
    Authentication,
    AuthenticatedValidation,
    PersistedRuntimeRead,
}

#[tokio::test]
async fn resume_survivor_errors_retain_all_durable_authority() {
    for fault in [
        ResumeSurvivorFault::RuntimeRead,
        ResumeSurvivorFault::RuntimeValidation,
        ResumeSurvivorFault::Authentication,
        ResumeSurvivorFault::AuthenticatedValidation,
        ResumeSurvivorFault::PersistedRuntimeRead,
    ] {
        assert_resume_survivor_fault(fault).await;
    }
}

async fn assert_resume_survivor_fault(fault: ResumeSurvivorFault) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::Survivor);
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);
    let mut server = None;
    let mut publication = None;
    match fault {
        ResumeSurvivorFault::RuntimeRead => {
            corrupt_resume_json(&host.paths.runtime_record(manifest.terminal_id));
        }
        ResumeSurvivorFault::RuntimeValidation => {
            runtime.command.push_str(RESUME_CONVERGENCE_FIELD_SUFFIX);
            host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)
                .unwrap();
        }
        ResumeSurvivorFault::Authentication => {}
        ResumeSurvivorFault::AuthenticatedValidation => {
            let (listener, guard) =
                publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
            let mut invalid = runtime.clone();
            invalid.command.push_str(RESUME_CONVERGENCE_FIELD_SUFFIX);
            server = Some(tokio::spawn(serve_resume_auth_once(listener, invalid)));
            publication = Some(guard);
        }
        ResumeSurvivorFault::PersistedRuntimeRead => {
            let (listener, guard) =
                publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
            server = Some(tokio::spawn(serve_auth_after_runtime_corruption(
                listener,
                runtime.clone(),
                host.paths.runtime_record(manifest.terminal_id),
            )));
            publication = Some(guard);
        }
    }
    let result = if matches!(fault, ResumeSurvivorFault::RuntimeRead) {
        host.finish_or_retain_planned_resume(manifest, &transaction)
            .await
    } else {
        host.retain_planned_resume_survivor(manifest, &transaction, runtime)
            .await
    };

    if let Some(server) = server {
        server.await.unwrap();
    }
    drop(publication);
    assert_resume_failure_retains(&host, &transaction, result);
    assert_survivor_authority_retained(&host, transaction.terminal_id);
}
