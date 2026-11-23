use super::*;

#[path = "../test_support/runtime_retirement.rs"]
mod support;
use support::Fixture;

const FIRST_RECORD: usize = 0;
const NEXT_GENERATION: u64 = 1;
const INTERRUPTED_PUBLICATION: &str = "interrupt after Claude custody settlement";

#[test]
fn verified_retirement_settles_claude_before_pty_receipt_and_unblocks_admission() {
    let fixture = Fixture::new();
    fixture.install_ledger();
    let settings = fixture
        .paths
        .session_dir(fixture.closed.runtime.terminal_id)
        .join(crate::claude_settings_artifact::DIRECTORY_NAME);
    assert!(crate::claude_custody::admit(&settings).is_err());
    fixture
        .publish(|authority| {
            assert_eq!(fixture.current(), fixture.armed);
            settle_runtime_retirement(authority)?;
            assert_eq!(
                fixture.ledger().records[FIRST_RECORD].state,
                CustodyState::Settled
            );
            assert_eq!(fixture.current(), fixture.armed);
            Ok(())
        })
        .unwrap();
    assert!(fixture.current().retired_lease().is_some());
    crate::claude_custody::admit(&settings).unwrap();
    assert!(fixture.ledger().records.is_empty());
}

#[test]
fn crash_between_provider_settlement_and_pty_publication_replays() {
    let fixture = Fixture::new();
    fixture.install_ledger();
    assert!(
        fixture
            .publish(|authority| {
                settle_runtime_retirement(authority)?;
                anyhow::bail!(INTERRUPTED_PUBLICATION)
            })
            .is_err()
    );
    assert_eq!(fixture.current(), fixture.armed);
    assert_eq!(
        fixture.ledger().records[FIRST_RECORD].state,
        CustodyState::Settled
    );
    fixture.publish(settle_runtime_retirement).unwrap();
    fixture.publish(settle_runtime_retirement).unwrap();
    assert!(fixture.current().retired_lease().is_some());
}

#[test]
fn exact_worker_scope_does_not_settle_another_generation() {
    let fixture = Fixture::new();
    let mut ledger = fixture.install_ledger();
    ledger.records[FIRST_RECORD].spec.worker.start_time += NEXT_GENERATION;
    fixture
        .namespace
        .write_json(&fixture.ledger_path(), &ledger)
        .unwrap();
    fixture.publish(settle_runtime_retirement).unwrap();
    assert_eq!(fixture.ledger(), ledger);
}

#[test]
fn absent_claude_ledger_remains_absent() {
    let fixture = Fixture::new();
    fixture.publish(settle_runtime_retirement).unwrap();
    assert!(!fixture.ledger_path().exists());
}

#[test]
fn completed_retirement_replay_preserves_the_claude_receipt_identity() {
    let fixture = Fixture::new();
    fixture.install_ledger();
    fixture.publish(settle_runtime_retirement).unwrap();
    let before: (CustodyLedger, _) = fixture
        .namespace
        .read_json_with_identity(&fixture.ledger_path())
        .unwrap();
    fixture.publish(settle_runtime_retirement).unwrap();
    let after: (CustodyLedger, _) = fixture
        .namespace
        .read_json_with_identity(&fixture.ledger_path())
        .unwrap();
    assert_eq!(before, after);
}

#[path = "runtime_retirement_adversarial.rs"]
mod adversarial;
