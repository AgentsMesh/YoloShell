use super::{filesystem::*, model::*};
use crate::*;
use yolo_hub_identity::HubId;

#[test]
fn promotion_retention_and_settlement_require_the_whole_original_intent() {
    let original = pending(HubId::new());
    for changed in conflicting_confirmations(&original) {
        for mutation in [
            (|journal: &mut CreationJournal, changed: PendingFinalization| {
                journal.retain_confirmed(changed)
            }) as fn(&mut CreationJournal, PendingFinalization) -> anyhow::Result<bool>,
            |journal, changed| journal.retain_intent(intent(&changed)),
            |journal, changed| journal.settle_intent(&intent(&changed)),
        ] {
            let fixture = Fixture::new();
            let mut journal = fixture.open();
            journal.retain_intent(intent(&original)).unwrap();
            let retained = fixture.stored();
            assert!(mutation(&mut journal, changed.clone()).is_err());
            assert!(journal.intents().is_err());
            assert_eq!(fixture.stored(), retained);
            journal.recover().unwrap();
            assert_eq!(journal.intents().unwrap(), [intent(&original)]);
            assert!(journal.pending().unwrap().is_empty());
        }
    }
}

#[test]
fn confirmed_request_alias_cannot_be_reintroduced_as_an_unconfirmed_intent() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let record = pending(HubId::new());
    journal.retain_confirmed(record.clone()).unwrap();
    let retained = fixture.stored();
    assert!(journal.retain_intent(intent(&record)).is_err());
    assert_eq!(fixture.stored(), retained);
    journal.recover().unwrap();
    assert_eq!(journal.pending().unwrap(), [record]);
}

#[test]
fn identical_replacement_and_namespace_replacement_never_authorize_intent_mutation() {
    for replace_namespace in [false, true] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let record = pending(HubId::new());
        journal.retain_intent(intent(&record)).unwrap();
        let retained = fixture.stored();
        if replace_namespace {
            std::fs::rename(&fixture.path, fixture.root.path().join(MOVED_DIRECTORY)).unwrap();
            yolo_paths::directory::PrivateDirectory::ensure(&fixture.path).unwrap();
        } else {
            fixture.replace(&retained);
        }
        assert!(journal.intents().is_err());
        assert!(journal.retain_confirmed(record.clone()).is_err());
        assert!(journal.settle_intent(&intent(&record)).is_err());
        assert!(journal.recover().is_err());
        if replace_namespace {
            assert!(!fixture.file().exists());
        } else {
            assert_eq!(fixture.stored(), retained);
        }
    }
}
