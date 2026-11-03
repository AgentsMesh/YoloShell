use crate::host::tests::*;
use super::*;

const SURVIVOR_MANIFEST_FAILURE_NAME: &str = "survivor-manifest-failure";
const SURVIVOR_CATALOG_FAILURE_NAME: &str = "survivor-catalog-failure";

#[tokio::test]
async fn survivor_manifest_failure_publishes_no_receipt_and_remains_retryable() {
    let fixture = survivor_creation_fixture(SURVIVOR_MANIFEST_FAILURE_NAME).await;
    let mut server = CleanupTerminationServer::start(&fixture);
    let terminal_id = fixture.binding.result.terminal_id;
    let manifest_path = fixture.host.paths.manifest(terminal_id);
    let original: TerminalManifest = read_json(&manifest_path).unwrap();
    std::fs::remove_file(&manifest_path).unwrap();
    std::fs::create_dir(&manifest_path).unwrap();

    let error = server.request(&fixture).await.unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    assert_transition_ledger(&fixture.ledger_path, &[fixture.binding.clone()]);
    assert!(
        fixture
            .host
            .creation_receipts
            .lookup_binding(&fixture.binding)
            .unwrap()
            .is_none()
    );

    std::fs::remove_dir(&manifest_path).unwrap();
    atomic_write_json(&manifest_path, &original).unwrap();
    assert_eq!(
        server.request(&fixture).await.unwrap(),
        CleanupTerminationOutcome::Survivor
    );
    let _receipt =
        assert_survivor_completion(&fixture.host, &fixture.binding, &fixture.ledger_path);
    assert_uncertain_survivor_recovery(&fixture.host, terminal_id);
    server.finish().await;
}

#[tokio::test]
async fn survivor_catalog_failure_publishes_no_receipt_and_remains_retryable() {
    let fixture = survivor_creation_fixture(SURVIVOR_CATALOG_FAILURE_NAME).await;
    let mut server = CleanupTerminationServer::start(&fixture);
    let terminal_id = fixture.binding.result.terminal_id;
    *lock_recover(&fixture.host.catalog_test_probes.upsert_failure) = true;

    let error = server.request(&fixture).await.unwrap_err();
    assert_eq!(error.code, crate::protocol::error_code::INTERNAL);
    assert_transition_ledger(&fixture.ledger_path, &[fixture.binding.clone()]);
    assert!(
        fixture
            .host
            .creation_receipts
            .lookup_binding(&fixture.binding)
            .unwrap()
            .is_none()
    );
    let durable: TerminalManifest = read_json(&fixture.host.paths.manifest(terminal_id)).unwrap();
    let catalog = lock_recover(&fixture.host.catalog)
        .get(terminal_id)
        .unwrap()
        .unwrap();
    assert_eq!(durable.recovery_state, RecoveryState::Uncertain);
    assert_ne!(catalog.recovery_state, RecoveryState::Uncertain);

    assert_eq!(
        server.request(&fixture).await.unwrap(),
        CleanupTerminationOutcome::Survivor
    );
    let _receipt =
        assert_survivor_completion(&fixture.host, &fixture.binding, &fixture.ledger_path);
    assert_uncertain_survivor_recovery(&fixture.host, terminal_id);
    server.finish().await;
}
