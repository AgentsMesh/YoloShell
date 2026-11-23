use crate::host::tests::*;
use super::*;

const READY_REARM_PROVIDER_PID: u32 = 51_902;
const READY_REARM_PROVIDER_START_TIME: u64 = 61_902;
const READY_REARM_TIMESTAMP_STEP: i64 = 1;

fn ready_rearm_case(
    host: &Host,
    manifest: &TerminalManifest,
    provider: bool,
) -> (ResumeTransaction, RuntimeRecord) {
    let transaction = if provider {
        provider_resume_fixture(host, manifest).2
    } else {
        recreate_survivor_transaction(host, manifest, Uuid::new_v4())
    };
    let delivery = if provider {
        BootstrapDelivery::Delivered
    } else {
        BootstrapDelivery::NotRequired
    };
    let runtime = planned_resume_runtime(
        host,
        &transaction,
        RESUME_TEST_DEAD_WORKER_PID,
        RESUME_TEST_PROCESS_START_TIME,
        RESUME_TEST_DEAD_CHILD_PID,
        RESUME_TEST_PROCESS_START_TIME,
        delivery,
    );
    (transaction, runtime)
}

fn attach_ready_provider_receipt(transaction: &mut ResumeTransaction, runtime: &RuntimeRecord) {
    let ResumeTarget::Provider { binding, .. } = &transaction.target else {
        panic!("provider resume target");
    };
    let mut observed = binding.clone();
    observed.process_pid = Some(READY_REARM_PROVIDER_PID);
    observed.process_start_time = Some(READY_REARM_PROVIDER_START_TIME);
    observed.state = crate::agent::AgentState::Running;
    observed.updated_at_ms = observed
        .updated_at_ms
        .saturating_add(READY_REARM_TIMESTAMP_STEP);
    let observed_at_ms = observed.updated_at_ms;
    let authority =
        ProviderResumeAuthority::new(transaction, runtime, observed, observed_at_ms).unwrap();
    transaction.provider_completion = Some(authority);
    transaction.validate_for(runtime.terminal_id).unwrap();
}

#[tokio::test]
async fn ready_resume_rearms_after_reboot_before_receipt_or_manifest_commit() {
    for provider in [false, true] {
        let directory = tempdir().unwrap();
        let host = unspawnable_resume_host(&directory);
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = RecoveryState::Restoring;
        let (transaction, mut runtime) = ready_rearm_case(&host, &manifest, provider);
        runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
        persist_survivor_resume(&host, &manifest, &transaction, &runtime);

        assert_eq!(
            host.dead_resume_terminal_evidence(&manifest, &transaction)
                .unwrap(),
            Some(runtime.clone())
        );
        let replacement = host
            .rearm_dead_resume_terminal(&mut manifest, &transaction)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(replacement.previous_runtime_epoch, runtime.runtime_epoch);
        assert_eq!(replacement.target, transaction.target);
        assert!(replacement.provider_completion.is_none());
    }
}

#[tokio::test]
async fn same_boot_ready_resume_requires_exact_custody_settlement() {
    for provider in [false, true] {
        let directory = tempdir().unwrap();
        let host = unspawnable_resume_host(&directory);
        let mut manifest = fixture_manifest(directory.path());
        manifest.recovery_state = RecoveryState::Restoring;
        let (transaction, runtime) = ready_rearm_case(&host, &manifest, provider);
        persist_survivor_resume(&host, &manifest, &transaction, &runtime);

        assert!(
            host.dead_resume_terminal_evidence(&manifest, &transaction)
                .unwrap()
                .is_none()
        );
        assert!(
            host.rearm_dead_resume_terminal(&mut manifest, &transaction)
                .await
                .unwrap()
                .is_none()
        );
    }
}

#[tokio::test]
async fn ready_provider_rearm_drops_the_retired_completion_receipt() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let mut manifest = fixture_manifest(directory.path());
    manifest.recovery_state = RecoveryState::Restoring;
    let (mut transaction, mut runtime) = ready_rearm_case(&host, &manifest, true);
    runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
    attach_ready_provider_receipt(&mut transaction, &runtime);
    persist_survivor_resume(&host, &manifest, &transaction, &runtime);

    let replacement = host
        .rearm_dead_resume_terminal(&mut manifest, &transaction)
        .await
        .unwrap()
        .unwrap();

    assert!(replacement.provider_completion.is_none());
    assert_ne!(
        replacement.transaction_digest,
        transaction.transaction_digest
    );
}

#[tokio::test]
async fn committed_ready_resume_cleans_up_without_rearming_a_dead_runtime() {
    for provider in [false, true] {
        let directory = tempdir().unwrap();
        let host = unspawnable_resume_host(&directory);
        let mut manifest = fixture_manifest(directory.path());
        let (mut transaction, mut runtime) = ready_rearm_case(&host, &manifest, provider);
        runtime.boot_id = RESUME_TEST_PREVIOUS_BOOT_ID.into();
        if provider {
            attach_ready_provider_receipt(&mut transaction, &runtime);
        }
        manifest.recovery_state = transaction.target.completed_state();
        persist_host_session(&host, &manifest, &runtime);
        host.publish_resume_transaction(&transaction).unwrap();

        let view = host
            .resume_session_locked(
                manifest.terminal_id,
                ResumeSelection::Preserve(ResumeIntent::Automatic),
            )
            .await
            .unwrap();

        assert_eq!(view.manifest.recovery_state, manifest.recovery_state);
        assert_eq!(
            read_json::<RuntimeRecord>(&host.paths.runtime_record(manifest.terminal_id)).unwrap(),
            runtime
        );
        assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
        assert!(!host.paths.worker_launch(manifest.terminal_id).exists());
    }
}
