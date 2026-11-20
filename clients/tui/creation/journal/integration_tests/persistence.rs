use super::{filesystem::*, model::*};
use crate::*;
use std::os::unix::fs::PermissionsExt;
use yolo_hub_identity::HubId;

#[test]
fn reopened_journal_retains_exact_remote_receipts_until_correlated_acknowledgement() {
    let fixture = Fixture::new();
    let record = pending(HubId::new());
    let mut journal = fixture.open();
    assert!(journal.pending().unwrap().is_empty());
    assert!(journal.retain_confirmed(record.clone()).unwrap());
    assert!(!journal.retain_confirmed(record.clone()).unwrap());
    let persisted = fixture.stored();
    journal.verify().unwrap();
    drop(journal);
    let mut journal = fixture.open();
    assert_eq!(journal.pending().unwrap(), [record.clone()]);
    // A lost Finalize acknowledgement cannot remove evidence: reopening retains it.
    assert!(!journal.retain_confirmed(record.clone()).unwrap());
    assert_eq!(fixture.stored(), persisted);
    assert!(journal.acknowledge(&record).unwrap());
    assert!(!journal.acknowledge(&record).unwrap());
    drop(journal);
    assert!(fixture.open().pending().unwrap().is_empty());
    assert_eq!(
        std::fs::metadata(fixture.file())
            .unwrap()
            .permissions()
            .mode()
            & MODE_MASK,
        PRIVATE_MODE
    );
}

#[test]
fn acknowledging_another_result_never_discards_a_retained_creation() {
    let fixture = Fixture::new();
    let record = pending(HubId::new());
    let mut journal = fixture.open();
    journal.retain_confirmed(record.clone()).unwrap();
    let before = fixture.stored();
    let mut wrong = record.clone();
    wrong.creation.creation_capability = yolo_hub_terminal_protocol::SecretCapability::generate();
    assert!(journal.acknowledge(&wrong).is_err());
    assert!(journal.pending().is_err());
    assert_eq!(fixture.stored(), before);
    drop(journal);
    assert_eq!(fixture.open().pending().unwrap(), [record]);
}

#[test]
fn changed_request_or_invalid_origin_poison_without_overwriting_evidence() {
    for change in [
        (|record: &mut PendingFinalization| {
            change_request(record, |fields| fields.cwd = OTHER_REQUEST_DIRECTORY.into())
        }) as fn(&mut PendingFinalization),
        |record: &mut PendingFinalization| record.origin = HubId::new(),
    ] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let mut record = pending(HubId::new());
        journal.retain_confirmed(record.clone()).unwrap();
        let before = fixture.stored();
        change(&mut record);
        assert!(journal.retain_confirmed(record).is_err());
        assert!(journal.verify().is_err());
        assert_eq!(fixture.stored(), before);
    }
}

#[test]
fn full_journal_preserves_all_receipts_and_reopen_can_drain_them() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let records: Vec<_> = (0..MAX_PENDING_CREATIONS)
        .map(|_| pending(HubId::new()))
        .collect();
    for record in &records {
        journal.retain_confirmed(record.clone()).unwrap();
    }
    assert!(
        !journal
            .retain_confirmed(records[FIRST_INDEX].clone())
            .unwrap()
    );
    assert!(journal.retain_confirmed(pending(HubId::new())).is_err());
    assert!(journal.pending().is_err());
    drop(journal);
    let mut journal = fixture.open();
    assert_eq!(journal.pending().unwrap(), records);
    assert!(journal.acknowledge(&records[FIRST_INDEX]).unwrap());
    journal.retain_confirmed(pending(HubId::new())).unwrap();
    assert_eq!(journal.pending().unwrap().len(), MAX_PENDING_CREATIONS);
}

#[test]
fn exhausted_revision_rejects_mutation_but_keeps_the_last_durable_record() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let record = pending(HubId::new());
    journal.retain_confirmed(record.clone()).unwrap();
    drop(journal);
    let mut value = fixture.stored();
    value[REVISION_FIELD] = u64::MAX.into();
    fixture.replace(&value);
    let mut journal = fixture.open();
    assert!(!journal.retain_confirmed(record.clone()).unwrap());
    assert!(journal.acknowledge(&record).is_err());
    assert_eq!(fixture.stored(), value);
}
