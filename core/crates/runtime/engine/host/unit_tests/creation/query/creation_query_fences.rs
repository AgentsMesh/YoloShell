use super::*;

#[tokio::test]
async fn query_does_not_heal_a_poisoned_or_changed_durable_ledger() {
    let fixture = QueryFixture::new();
    lock_recover(&fixture.host.creation_capabilities).poisoned = true;
    assert_eq!(
        fixture.observe(&fixture.request).await.unwrap_err().code,
        crate::protocol::error_code::INTERNAL
    );
    assert!(lock_recover(&fixture.host.creation_capabilities).poisoned);
    lock_recover(&fixture.host.creation_capabilities).poisoned = false;
    write_creation_ledger(&fixture.ledger_path, &CreationLedger::default()).unwrap();
    assert_eq!(
        fixture.observe(&fixture.request).await.unwrap_err().code,
        crate::protocol::error_code::INTERNAL
    );
    assert_eq!(
        lock_recover(&fixture.host.creation_capabilities)
            .bindings
            .len(),
        QUERY_ONLY_BINDING
    );
    assert!(
        read_creation_ledger::<CreationLedger>(&fixture.ledger_path)
            .unwrap()
            .unwrap()
            .bindings
            .is_empty()
    );
}

#[tokio::test]
async fn a_replaced_ledger_entry_cannot_supply_creation_authority() {
    let fixture = QueryFixture::new();
    let replacement = fixture.directory.path().join(QUERY_REPLACEMENT_FILE);
    std::fs::rename(&fixture.ledger_path, &replacement).unwrap();
    std::os::unix::fs::symlink(&replacement, &fixture.ledger_path).unwrap();
    assert_eq!(
        fixture.observe(&fixture.request).await.unwrap_err().code,
        crate::protocol::error_code::INTERNAL
    );
}

#[tokio::test]
async fn completed_receipt_conflicting_with_launching_phase_fails_closed() {
    let mut fixture = QueryFixture::new();
    fixture.phase(CreationPhase::Launching);
    let receipt = fixture.receipt(CreationCompletion::Finalized);
    fixture.publish(&receipt, true, true);
    assert_eq!(
        fixture.observe(&fixture.request).await.unwrap_err().code,
        crate::protocol::error_code::INTERNAL
    );
}

#[tokio::test]
async fn replaced_host_namespace_is_not_followed_or_repaired_by_query() {
    let fixture = QueryFixture::new();
    let home = fixture.host.paths.home().to_path_buf();
    let retained = fixture.directory.path().join(QUERY_REPLACED_HOME_DIRECTORY);
    std::fs::rename(&home, &retained).unwrap();
    std::fs::create_dir(&home).unwrap();
    write_private_bytes(&home.join(QUERY_STAGE_FILE), QUERY_STAGE_BYTES);
    assert_eq!(
        fixture.observe(&fixture.request).await.unwrap_err().code,
        crate::protocol::error_code::INTERNAL
    );
    let displaced = fixture
        .directory
        .path()
        .join(QUERY_DISPLACED_HOME_DIRECTORY);
    std::fs::rename(&home, &displaced).unwrap();
    std::fs::rename(&retained, &home).unwrap();
}

#[tokio::test]
async fn a_busy_creation_ledger_expires_without_becoming_missing_or_writing() {
    let mut fixture = QueryFixture::new();
    fixture.host.worker_ready_timeout = QUERY_LOCK_BUDGET;
    let _held = lock_recover(&fixture.host.creation_capabilities);
    assert_eq!(
        fixture.observe(&fixture.request).await.unwrap_err().code,
        crate::protocol::error_code::DELIVERY_UNKNOWN
    );
}
