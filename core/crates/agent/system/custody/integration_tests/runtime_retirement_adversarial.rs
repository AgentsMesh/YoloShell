use super::*;

const PRIMARY_GUARDIAN: usize = 0;
const FOREIGN_BOOT: &str = "foreign-runtime-retirement-boot";

#[test]
fn live_or_unknown_claude_participant_preserves_both_ledgers() {
    for relation in [
        crate::process::ProcessIdentityRelation::Matching,
        crate::process::ProcessIdentityRelation::Unknown,
    ] {
        let fixture = Fixture::new();
        let ledger = fixture.install_ledger();
        let guardian = &ledger.records[FIRST_RECORD].spec.guardians[PRIMARY_GUARDIAN];
        let _observation = crate::process::install_test_process_identity_relation(
            guardian.pid,
            guardian.start_time,
            relation,
        );
        assert!(fixture.publish(settle_runtime_retirement).is_err());
        assert_eq!(fixture.ledger(), ledger);
        assert_eq!(fixture.current(), fixture.armed);
    }
}

#[test]
fn mixed_boot_participant_cannot_borrow_current_runtime_retirement() {
    let fixture = Fixture::new();
    let mut ledger = fixture.install_ledger();
    ledger.records[FIRST_RECORD].spec.target.boot_id = FOREIGN_BOOT.into();
    fixture
        .namespace
        .write_json(&fixture.ledger_path(), &ledger)
        .unwrap();
    assert!(fixture.publish(settle_runtime_retirement).is_err());
    assert_eq!(fixture.ledger(), ledger);
    assert_eq!(fixture.current(), fixture.armed);
}

#[test]
fn same_value_runtime_replacement_revokes_settlement_authority() {
    let fixture = Fixture::new();
    let ledger = fixture.install_ledger();
    assert!(
        fixture
            .publish(|authority| {
                let path = fixture
                    .paths
                    .runtime_record(fixture.closed.runtime.terminal_id);
                let runtime: crate::model::RuntimeRecord = fixture.namespace.read_json(&path)?;
                crate::paths::atomic_write_json(&path, &runtime)?;
                settle_runtime_retirement(authority)
            })
            .is_err()
    );
    assert_eq!(fixture.ledger(), ledger);
    assert_eq!(fixture.current(), fixture.armed);
}

#[test]
fn unrelated_settled_pty_cannot_authorize_this_claude_worker() {
    let fixture = Fixture::new();
    let ledger = fixture.install_ledger();
    let unrelated = crate::process::prepared_retirement_custody_fixture();
    let guard = yolo_paths::durable_mutation::SessionDurableMutationGuard::acquire_participant(
        &fixture.paths,
        &fixture.namespace,
        fixture.closed.runtime.terminal_id,
    )
    .unwrap();
    assert!(
        crate::custody_retirement::publish_exact_with_guard(
            &guard,
            &fixture.paths,
            &unrelated.custody,
            &fixture.closed.runtime,
            fixture.armed.armed_lease().unwrap(),
            settle_runtime_retirement,
        )
        .is_err()
    );
    assert_eq!(fixture.ledger(), ledger);
    assert_eq!(fixture.current(), fixture.armed);
}

#[test]
fn same_value_pty_replacement_revokes_settlement_authority() {
    let fixture = Fixture::new();
    let ledger = fixture.install_ledger();
    assert!(
        fixture
            .publish(|authority| {
                let path = fixture.paths.pty_custody(fixture.closed.runtime.terminal_id);
                crate::paths::atomic_write_json(&path, &fixture.armed)?;
                settle_runtime_retirement(authority)
            })
            .is_err()
    );
    assert_eq!(fixture.ledger(), ledger);
    assert_eq!(fixture.current(), fixture.armed);
}

#[test]
fn pending_intent_prevents_claude_and_pty_retirement() {
    let fixture = Fixture::new();
    let ledger = fixture.install_ledger();
    let intent = fixture
        .paths
        .reboot_migration_intent(fixture.closed.runtime.terminal_id);
    fixture
        .namespace
        .write_json(&intent, &serde_json::Value::Null)
        .unwrap();
    assert!(fixture.publish(settle_runtime_retirement).is_err());
    assert_eq!(fixture.ledger(), ledger);
    assert_eq!(fixture.current(), fixture.armed);
}

#[test]
fn archived_manifest_rejects_provider_settlement() {
    let fixture = Fixture::new();
    let ledger = fixture.install_ledger();
    let path = fixture.paths.manifest(fixture.closed.runtime.terminal_id);
    let mut manifest: crate::model::TerminalManifest = fixture.namespace.read_json(&path).unwrap();
    manifest.archived = true;
    fixture.namespace.write_json(&path, &manifest).unwrap();
    assert!(fixture.publish(settle_runtime_retirement).is_err());
    assert_eq!(fixture.ledger(), ledger);
    assert_eq!(fixture.current(), fixture.armed);
}

#[test]
fn corrupt_ledger_is_not_replaced_by_a_successful_pty_receipt() {
    let fixture = Fixture::new();
    fixture
        .namespace
        .write_json(&fixture.ledger_path(), &serde_json::Value::Null)
        .unwrap();
    assert!(fixture.publish(settle_runtime_retirement).is_err());
    let ledger: serde_json::Value = fixture.namespace.read_json(&fixture.ledger_path()).unwrap();
    assert_eq!(ledger, serde_json::Value::Null);
    assert_eq!(fixture.current(), fixture.armed);
}
