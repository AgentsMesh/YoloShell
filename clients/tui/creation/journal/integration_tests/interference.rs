use super::{filesystem::*, model::*};
use crate::*;
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;

#[test]
fn byte_identical_external_replacement_poisons_reads_and_writes_without_adoption() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let record = pending(HubId::new());
    journal.retain_confirmed(record.clone()).unwrap();
    let before = fixture.stored();
    fixture.replace(&before);
    assert!(journal.pending().is_err());
    assert!(journal.retain_confirmed(record.clone()).is_err());
    assert!(journal.acknowledge(&record).is_err());
    assert_eq!(fixture.stored(), before);
}

#[test]
fn removed_record_and_renamed_namespace_cannot_be_recreated_by_retained_owner() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let record = pending(HubId::new());
    journal.retain_confirmed(record.clone()).unwrap();
    std::fs::remove_file(fixture.file()).unwrap();
    assert!(journal.retain_confirmed(record).is_err());
    assert!(!fixture.file().exists());
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    std::fs::rename(&fixture.path, fixture.root.path().join(MOVED_DIRECTORY)).unwrap();
    PrivateDirectory::ensure(&fixture.path).unwrap();
    assert!(journal.retain_confirmed(pending(HubId::new())).is_err());
    assert!(journal.verify().is_err());
    assert!(!fixture.file().exists());
}

#[test]
fn replacing_record_with_symlink_cannot_redirect_acknowledgement() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let record = pending(HubId::new());
    journal.retain_confirmed(record.clone()).unwrap();
    let other = fixture.root.path().join(OTHER_DIRECTORY);
    std::fs::rename(fixture.file(), &other).unwrap();
    let before = std::fs::read(&other).unwrap();
    std::os::unix::fs::symlink(&other, fixture.file()).unwrap();
    assert!(journal.acknowledge(&record).is_err());
    assert_eq!(std::fs::read(other).unwrap(), before);
}

#[test]
fn wrong_acknowledgement_origin_or_cross_origin_request_alias_never_removes_receipt() {
    for same_token in [false, true] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let original = pending(HubId::new());
        journal.retain_confirmed(original.clone()).unwrap();
        let before = fixture.stored();
        let mut wrong = pending(HubId::new());
        change_request(&mut wrong, |fields| {
            if same_token {
                fields.creation_token = original.request.fields().creation_token;
            } else {
                fields.request_id = original.request.fields().request_id;
            }
        });
        assert!(journal.acknowledge(&wrong).is_err());
        assert_eq!(fixture.stored(), before);
        drop(journal);
        let mut journal = fixture.open();
        assert!(journal.retain_confirmed(wrong).is_err());
        assert_eq!(fixture.stored(), before);
    }
}
