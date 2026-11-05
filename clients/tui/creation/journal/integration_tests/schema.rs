use super::{filesystem::*, model::*};
use crate::*;
use yolo_hub_identity::HubId;

#[test]
fn unsupported_journal_is_rejected_without_rewriting_pending_evidence() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    journal.retain_confirmed(pending(HubId::new())).unwrap();
    drop(journal);
    let mut value = fixture.stored();
    value[SCHEMA_FIELD] = UNSUPPORTED_SCHEMA_VERSION.into();
    for include_intents in [true, false] {
        if !include_intents {
            value.as_object_mut().unwrap().remove(INTENTS_FIELD);
        }
        fixture.replace(&value);
        let bytes = std::fs::read(fixture.file()).unwrap();
        assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
        assert_eq!(std::fs::read(fixture.file()).unwrap(), bytes);
    }
}
