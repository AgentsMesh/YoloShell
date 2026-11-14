use crate::host::tests::*;
#[derive(Clone, Copy)]
enum PlannedCompletionFault {
    RuntimeRead,
    RuntimeValidation,
    Authentication,
    AuthenticatedValidation,
    PersistedRuntimeRead,
}

#[tokio::test]
async fn resume_planned_completion_errors_preserve_durable_authority() {
    for fault in [
        PlannedCompletionFault::RuntimeRead,
        PlannedCompletionFault::RuntimeValidation,
        PlannedCompletionFault::Authentication,
        PlannedCompletionFault::AuthenticatedValidation,
        PlannedCompletionFault::PersistedRuntimeRead,
    ] {
        assert_planned_completion_fault(fault).await;
    }
}

async fn assert_planned_completion_fault(fault: PlannedCompletionFault) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(Uuid::new_v4(), &launch);
    let mut runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::NotRequired);
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    let mut server = None;
    let mut publication = None;
    match fault {
        PlannedCompletionFault::RuntimeRead => {
            corrupt_resume_json(&host.paths.runtime_record(manifest.terminal_id));
        }
        PlannedCompletionFault::RuntimeValidation => {
            runtime.command.push_str(RESUME_CONVERGENCE_FIELD_SUFFIX);
            host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)
                .unwrap();
        }
        PlannedCompletionFault::Authentication => {}
        PlannedCompletionFault::AuthenticatedValidation => {
            let (listener, guard) =
                publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
            let mut invalid = runtime.clone();
            invalid.command.push_str(RESUME_CONVERGENCE_FIELD_SUFFIX);
            server = Some(tokio::spawn(serve_resume_auth_once(listener, invalid)));
            publication = Some(guard);
        }
        PlannedCompletionFault::PersistedRuntimeRead => {
            let (listener, guard) =
                publish_test_socket(&host.socket_namespace, &runtime.socket_path).await;
            server = Some(tokio::spawn(serve_auth_after_runtime_corruption(
                listener,
                runtime,
                host.paths.runtime_record(manifest.terminal_id),
            )));
            publication = Some(guard);
        }
    }

    let result = host.finish_planned_resume(manifest, &transaction).await;

    if let Some(server) = server {
        server.await.unwrap();
    }
    drop(publication);
    assert_resume_failure_retains(&host, &transaction, result);
}

#[derive(Clone, Copy)]
enum CommittedCompletionFault {
    RuntimeRead,
    RuntimeValidation,
    BindingCompletion,
}

#[tokio::test]
async fn resume_committed_completion_errors_preserve_durable_authority() {
    for fault in [
        CommittedCompletionFault::RuntimeRead,
        CommittedCompletionFault::RuntimeValidation,
        CommittedCompletionFault::BindingCompletion,
    ] {
        assert_committed_completion_fault(fault).await;
    }
}

async fn assert_committed_completion_fault(fault: CommittedCompletionFault) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    let transaction = match fault {
        CommittedCompletionFault::BindingCompletion => {
            binding_clear_transaction(&host, &manifest, None)
        }
        CommittedCompletionFault::RuntimeRead | CommittedCompletionFault::RuntimeValidation => {
            shell_resume_transaction(Uuid::new_v4(), &shell_resume_launch(&host, &manifest))
        }
    };
    manifest.recovery_state = transaction.target.completed_state();
    let mut runtime = live_resume_runtime(&host, &transaction, BootstrapDelivery::NotRequired);
    persist_host_session(&host, &manifest, &runtime);
    host.publish_resume_transaction(&transaction).unwrap();
    match fault {
        CommittedCompletionFault::RuntimeRead => {
            corrupt_resume_json(&host.paths.runtime_record(manifest.terminal_id));
        }
        CommittedCompletionFault::RuntimeValidation => {
            runtime.command.push_str(RESUME_CONVERGENCE_FIELD_SUFFIX);
            host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &runtime)
                .unwrap();
        }
        CommittedCompletionFault::BindingCompletion => {
            let marker =
                binding_clear_marker(&transaction, ResumeBindingClearPhase::Pending).unwrap();
            host.write_namespace_json(
                &host.paths.resume_binding_clear(manifest.terminal_id),
                &marker,
            )
            .unwrap();
        }
    }

    let result = host.finish_committed_resume(manifest, &transaction).await;

    assert_resume_failure_retains(&host, &transaction, result);
}
