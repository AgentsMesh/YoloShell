use super::{filesystem::*, model::*};
use crate::*;
use yolo_hub_identity::HubId;

#[test]
fn durable_intent_reopens_without_granting_another_initial_delivery() {
    let fixture = Fixture::new();
    let record = pending(HubId::new());
    let intent = intent(&record);
    let mut journal = fixture.open();
    assert!(journal.retain_intent(intent.clone()).unwrap());
    let retained = fixture.stored();
    drop(journal);
    let mut journal = fixture.open();
    assert_eq!(journal.intents().unwrap(), [intent.clone()]);
    assert!(journal.pending().unwrap().is_empty());
    assert!(!journal.retain_intent(intent.clone()).unwrap());
    assert_eq!(fixture.stored(), retained);
    // The caller has authenticated an exact terminal query result before settlement.
    assert!(journal.settle_intent(&intent).unwrap());
    assert!(!journal.settle_intent(&intent).unwrap());
    drop(journal);
    assert!(fixture.open().intents().unwrap().is_empty());
}

#[test]
fn promotion_is_one_revision_and_works_at_the_shared_capacity_limit() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let record = pending(HubId::new());
    journal.retain_intent(intent(&record)).unwrap();
    for _ in CAPACITY_SENTINEL as usize..MAX_PENDING_CREATIONS {
        journal.retain_confirmed(pending(HubId::new())).unwrap();
    }
    let before = fixture.stored()[REVISION_FIELD].as_u64().unwrap();
    assert!(journal.retain_confirmed(record.clone()).unwrap());
    assert_eq!(fixture.stored()[REVISION_FIELD], before + CAPACITY_SENTINEL);
    assert!(journal.intents().unwrap().is_empty());
    assert_eq!(journal.pending().unwrap().len(), MAX_PENDING_CREATIONS);
    assert!(!journal.retain_confirmed(record.clone()).unwrap());
    assert!(!journal.settle_intent(&intent(&record)).unwrap());
    assert!(journal.acknowledge(&record).unwrap());
    assert!(
        journal
            .retain_intent(intent(&pending(HubId::new())))
            .unwrap()
    );
    assert!(
        journal
            .retain_intent(intent(&pending(HubId::new())))
            .is_err()
    );
}

#[test]
fn confirmed_receipts_are_independent_of_unresolved_intents() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let intent = intent(&pending(HubId::new()));
    let confirmed = pending(HubId::new());
    journal.retain_intent(intent.clone()).unwrap();
    journal.retain_confirmed(confirmed.clone()).unwrap();
    journal.acknowledge(&confirmed).unwrap();
    assert_eq!(journal.intents().unwrap(), [intent]);
}
