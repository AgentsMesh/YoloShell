use crate::host::tests::*;
#[derive(Clone, Copy)]
enum ResumeContinuationFault {
    TransactionValidation,
    DurableManifest,
    TransactionLaunch,
    PublishedLaunchRead,
    PublishedLaunchValidation,
    RuntimeRead,
    PreviousRuntimeValidation,
    BindingStaging,
    PlannedRuntimeValidation,
}

#[tokio::test]
async fn resume_continuation_errors_preserve_durable_authority() {
    for fault in [
        ResumeContinuationFault::TransactionValidation,
        ResumeContinuationFault::DurableManifest,
        ResumeContinuationFault::TransactionLaunch,
        ResumeContinuationFault::PublishedLaunchRead,
        ResumeContinuationFault::PublishedLaunchValidation,
        ResumeContinuationFault::RuntimeRead,
        ResumeContinuationFault::PreviousRuntimeValidation,
        ResumeContinuationFault::BindingStaging,
        ResumeContinuationFault::PlannedRuntimeValidation,
    ] {
        assert_resume_continuation_fault(fault).await;
    }
}

async fn assert_resume_continuation_fault(fault: ResumeContinuationFault) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Uncertain;
    let previous = persist_dead_resume_session(&host, &manifest);
    let launch = shell_resume_launch(&host, &manifest);
    let transaction = shell_resume_transaction(previous.runtime_epoch, &launch);
    host.publish_resume_transaction(&transaction).unwrap();
    let mut supplied = transaction.clone();
    match fault {
        ResumeContinuationFault::TransactionValidation => {
            supplied.transaction_digest.clear();
        }
        ResumeContinuationFault::DurableManifest => {
            corrupt_resume_json(&host.paths.manifest(manifest.terminal_id));
        }
        ResumeContinuationFault::TransactionLaunch => {
            manifest.cwd = manifest.cwd.join(RESUME_CONVERGENCE_FIELD_SUFFIX);
            host.write_namespace_json(&host.paths.manifest(manifest.terminal_id), &manifest)
                .unwrap();
        }
        ResumeContinuationFault::PublishedLaunchRead => {
            host.publish_worker_launch(&transaction.launch).unwrap();
            corrupt_resume_json(&host.paths.worker_launch(manifest.terminal_id));
        }
        ResumeContinuationFault::PublishedLaunchValidation => {
            let foreign = shell_resume_launch(&host, &manifest);
            host.publish_worker_launch(&foreign).unwrap();
        }
        ResumeContinuationFault::RuntimeRead => {
            corrupt_resume_json(&host.paths.runtime_record(manifest.terminal_id));
        }
        ResumeContinuationFault::PreviousRuntimeValidation => {
            let mut invalid = previous;
            invalid.worker_start_time = RESUME_CONVERGENCE_INVALID_START_TIME;
            host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &invalid)
                .unwrap();
        }
        ResumeContinuationFault::BindingStaging => {
            write_unexpected_resume_binding_marker(&host, &manifest);
        }
        ResumeContinuationFault::PlannedRuntimeValidation => {
            let mut invalid =
                live_resume_runtime(&host, &transaction, BootstrapDelivery::NotRequired);
            invalid.command.push_str(RESUME_CONVERGENCE_FIELD_SUFFIX);
            host.write_namespace_json(&host.paths.runtime_record(manifest.terminal_id), &invalid)
                .unwrap();
        }
    }

    let corrupt = matches!(fault, ResumeContinuationFault::DurableManifest)
        .then(|| resume_manifest_identity(&host, manifest.terminal_id));
    let result = host.continue_resume_transaction(manifest, supplied).await;

    match corrupt {
        Some(identity) => {
            assert_resume_failure_retains_corruption(&host, &transaction, identity, result)
        }
        None => assert_resume_failure_retains(&host, &transaction, result),
    }
}
