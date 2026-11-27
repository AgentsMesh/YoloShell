use super::{filesystem::*, model::*};
use crate::{CreationJournal, JOURNAL_FILE};
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;

const HELD: &str = "held-original";
const DIRECTORY_FIELD: &str = "directory";

#[test]
fn replacement_at_original_path_cannot_reopen_uncertain_creation_history() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let confirmed = pending(HubId::new());
    journal.retain_intent(intent(&confirmed)).unwrap();
    let original = fixture.parent().path().to_owned();
    let held = fixture.root.path().join(HELD);
    drop(journal);
    std::fs::rename(&original, &held).unwrap();
    let replacement = PrivateDirectory::ensure(&original).unwrap();
    std::fs::copy(held.join(JOURNAL_FILE), fixture.file()).unwrap();
    let before = std::fs::read(fixture.file()).unwrap();
    assert!(CreationJournal::open(replacement, fixture.entry).is_err());
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
}

#[test]
fn valid_foreign_durable_proof_cannot_replace_client_ownership() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    journal.retain_confirmed(pending(HubId::new())).unwrap();
    drop(journal);
    let mut document = fixture.stored();
    document[DIRECTORY_FIELD] = serde_json::to_value(directory_identity()).unwrap();
    fixture.replace(&document);
    let before = std::fs::read(fixture.file()).unwrap();
    assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
    assert_eq!(std::fs::read(fixture.file()).unwrap(), before);
}
