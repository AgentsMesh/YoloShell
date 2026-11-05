use crate::constants::PAIRING_LEDGER_FILE;
use crate::invites::{InviteLedger, InviteLedgerError};
use uuid::Uuid;
use yolo_paths::directory::PrivateDirectory;

fn root(directory: &tempfile::TempDir) -> PrivateDirectory {
    PrivateDirectory::ensure(&directory.path().join("hub")).unwrap()
}

#[test]
fn consumed_invites_survive_reopen_and_reject_replay() {
    let directory = tempfile::tempdir().unwrap();
    let invite_id = Uuid::new_v4();
    {
        let mut ledger = InviteLedger::open(&root(&directory)).unwrap();
        assert_eq!(ledger.consume(invite_id, 1_000_000, 500), Ok(()));
    }
    let mut ledger = InviteLedger::open(&root(&directory)).unwrap();
    assert_eq!(
        ledger.consume(invite_id, 1_000_000, 600),
        Err(InviteLedgerError::Consumed)
    );
}

#[test]
fn expired_invites_are_pruned_before_consumption() {
    let directory = tempfile::tempdir().unwrap();
    let invite_id = Uuid::new_v4();
    let mut ledger = InviteLedger::open(&root(&directory)).unwrap();
    assert_eq!(ledger.consume(invite_id, 1_000, 500), Ok(()));
    assert_eq!(
        ledger.consume(invite_id, 1_000, 500),
        Err(InviteLedgerError::Consumed)
    );
    assert_eq!(ledger.consume(invite_id, 1_000, 2_000_000), Ok(()));
}

#[test]
fn corrupt_ledgers_fail_closed() {
    let directory = tempfile::tempdir().unwrap();
    let root = root(&directory);
    let pairing = root.ensure_descendant(&root.path().join("pairing")).unwrap();
    std::fs::write(pairing.path().join(PAIRING_LEDGER_FILE), b"not json").unwrap();
    assert!(InviteLedger::open(&root).is_err());
}
