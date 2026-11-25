use crate::support::*;
use yolo_hub_identity::{TerminalAddress, TerminalId};
use yolo_hub_terminal_protocol::{CreateShellResult, SecretCapability};
use yolo_tui_creation_journal::{CreationJournal, JOURNAL_FILE, PendingFinalization};
use yolo_tui_tab::{TabStore, open_client_in};

#[test]
fn root_client_keeps_its_pending_creation_evidence_when_the_client_reopens() {
    let (_root, base, hub) = fixture();
    let root_client = TabStore::open(&base, hub).unwrap();
    let mut journal = CreationJournal::open(root_client.directory().unwrap(), hub).unwrap();
    let intent = intent(hub);
    journal.retain_intent(intent.clone()).unwrap();
    let before = std::fs::read(base.join(JOURNAL_FILE)).unwrap();
    drop(journal);
    drop(root_client);
    let reopened = open_client_in(&base, hub).unwrap();
    assert_eq!(reopened.directory().unwrap().path(), base);
    let journal = CreationJournal::open(reopened.directory().unwrap(), hub).unwrap();
    assert_eq!(journal.intents().unwrap(), vec![intent]);
    assert_eq!(std::fs::read(base.join(JOURNAL_FILE)).unwrap(), before);
}

#[test]
fn pending_child_journal_takes_priority_over_clean_default_state_without_replay() {
    let (_root, base, hub) = fixture();
    let first = open_client_in(&base, hub).unwrap();
    let second = open_client_in(&base, hub).unwrap();
    let second_path = second.directory().unwrap().path().to_owned();
    let mut journal = CreationJournal::open(second.directory().unwrap(), hub).unwrap();
    let unresolved = intent(hub);
    journal.retain_intent(unresolved.clone()).unwrap();
    let confirmation = intent(hub);
    let pending = PendingFinalization {
        origin: hub,
        request: confirmation.request,
        creation: CreateShellResult {
            address: TerminalAddress {
                hub_id: hub,
                terminal_id: TerminalId::new(),
            },
            creation_capability: SecretCapability::generate(),
        },
    };
    journal.retain_confirmed(pending.clone()).unwrap();
    let before = std::fs::read(second_path.join(JOURNAL_FILE)).unwrap();
    drop(journal);
    drop(second);
    drop(first);
    let recovered = open_client_in(&base, hub).unwrap();
    assert_eq!(recovered.directory().unwrap().path(), second_path);
    assert_eq!(
        std::fs::read(second_path.join(JOURNAL_FILE)).unwrap(),
        before
    );
    let journal = CreationJournal::open(recovered.directory().unwrap(), hub).unwrap();
    assert_eq!(journal.intents().unwrap(), vec![unresolved]);
    assert_eq!(journal.pending().unwrap(), vec![pending]);
    let independent = open_client_in(&base, hub).unwrap();
    assert_eq!(independent.directory().unwrap().path(), base);
}

#[test]
fn corrupt_recovery_journal_is_preserved_and_cannot_be_hidden_by_a_new_client() {
    let (_root, base, hub) = fixture();
    let first = open_client_in(&base, hub).unwrap();
    let second = open_client_in(&base, hub).unwrap();
    let journal = second.directory().unwrap().path().join(JOURNAL_FILE);
    yolo_paths::atomic_write_json(&journal, &INVALID_BYTES).unwrap();
    drop(second);
    drop(first);
    let before = std::fs::read(&journal).unwrap();
    assert!(open_client_in(&base, hub).is_err());
    assert_eq!(std::fs::read(journal).unwrap(), before);
}
