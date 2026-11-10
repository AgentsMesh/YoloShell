use crate::host::tests::*;
use super::*;

const RECONCILE_RESUME_FAILURE_PREVIOUS_BOOT: &str = "reconcile-resume-failure-previous-boot";

#[tokio::test]
async fn previous_boot_corrupt_binding_remains_durably_uncertain() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    persist_previous_boot_resume_session(&host, &manifest);
    write_private_bytes(
        &host.paths.agent_binding(manifest.terminal_id),
        AGENT_BINDING_WAVE5_CORRUPT,
    );

    let result = host.reconcile_manifest(manifest.clone()).await;

    assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(result.needs_recreate, usize::default());
    assert_reconcile_recovery_state(&host, manifest.terminal_id, RecoveryState::Uncertain).await;
    assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
}

#[tokio::test]
async fn published_automatic_resume_failure_preserves_transaction_and_uncertainty() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    let previous = persist_previous_boot_resume_session(&host, &manifest);
    let binding = completion_provider_binding(&directory, manifest.terminal_id);
    ProviderAuthorityPublisher::new()
        .publish(&host, &binding)
        .unwrap();

    let result = host.reconcile_manifest(manifest.clone()).await;

    assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(result.needs_recreate, usize::default());
    assert_reconcile_recovery_state(&host, manifest.terminal_id, RecoveryState::Uncertain).await;
    let transaction: ResumeTransaction =
        read_json(&host.paths.resume_transaction(manifest.terminal_id)).unwrap();
    assert_eq!(transaction.previous_runtime_epoch, previous.runtime_epoch);
    assert!(matches!(transaction.target, ResumeTarget::Provider { .. }));
    assert_eq!(transaction.intent, ResumeIntent::Automatic);
    assert!(host.paths.worker_launch(manifest.terminal_id).exists());
}

#[tokio::test]
async fn previous_boot_missing_binding_is_a_confirmed_terminal_recreation() {
    let directory = tempdir().unwrap();
    let host = unspawnable_resume_host(&directory);
    let manifest = fixture_manifest(directory.path());
    persist_previous_boot_resume_session(&host, &manifest);

    let result = host.reconcile_manifest(manifest.clone()).await;

    assert_eq!(result.unreachable, RECONCILIATION_FAILURE_COUNT);
    assert_eq!(result.needs_recreate, RECONCILIATION_FAILURE_COUNT);
    assert_reconcile_recovery_state(&host, manifest.terminal_id, RecoveryState::NeedsRecreate).await;
    assert!(!host.paths.resume_transaction(manifest.terminal_id).exists());
}

fn persist_previous_boot_resume_session(host: &Host, manifest: &TerminalManifest) -> RuntimeRecord {
    let mut runtime = dead_resume_runtime(host, manifest);
    runtime.boot_id = RECONCILE_RESUME_FAILURE_PREVIOUS_BOOT.into();
    persist_host_session(host, manifest, &runtime);
    runtime
}

async fn assert_reconcile_recovery_state(host: &Host, terminal_id: Uuid, expected: RecoveryState) {
    let durable: TerminalManifest = read_json(&host.paths.manifest(terminal_id)).unwrap();
    let catalog = host.catalog_get(terminal_id).await.unwrap().unwrap();
    assert_eq!(durable.recovery_state, expected);
    assert_eq!(catalog.recovery_state, expected);
}
