use super::{filesystem::*, model::*, recovery::fail_at};
use crate::{checkpoint::Checkpoint, *};
use yolo_hub_identity::HubId;

#[test]
fn interrupted_promotion_recovers_the_intent_or_receipt_never_an_empty_gap() {
    for stage in [
        Checkpoint::BeforeCommit,
        Checkpoint::CommitObserved,
        Checkpoint::WriteAcknowledged,
    ] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let record = pending(HubId::new());
        journal.retain_intent(intent(&record)).unwrap();
        let failure = fail_at(stage);
        assert!(journal.retain_confirmed(record.clone()).is_err());
        let committed = fixture.stored();
        drop(failure);
        assert!(journal.intents().is_err());
        journal.recover().unwrap();
        if stage == Checkpoint::BeforeCommit {
            assert_eq!(journal.intents().unwrap(), [intent(&record)]);
            assert!(journal.pending().unwrap().is_empty());
        } else {
            assert!(journal.intents().unwrap().is_empty());
            assert_eq!(journal.pending().unwrap(), [record.clone()]);
            assert!(!journal.retain_confirmed(record).unwrap());
        }
        assert_eq!(fixture.stored(), committed);
    }
}

#[test]
fn lost_intent_write_ack_recovers_without_authorizing_a_second_initial_create() {
    for stage in [Checkpoint::CommitObserved, Checkpoint::WriteAcknowledged] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let intent = intent(&pending(HubId::new()));
        let failure = fail_at(stage);
        assert!(journal.retain_intent(intent.clone()).is_err());
        drop(failure);
        journal.recover().unwrap();
        assert_eq!(journal.intents().unwrap(), [intent.clone()]);
        assert!(!journal.retain_intent(intent).unwrap());
    }
}

#[test]
fn settlement_lost_ack_recovers_only_the_observed_exact_removal() {
    for replacement in [false, true] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let intent = intent(&pending(HubId::new()));
        journal.retain_intent(intent.clone()).unwrap();
        let failure = fail_at(Checkpoint::WriteAcknowledged);
        assert!(journal.settle_intent(&intent).is_err());
        drop(failure);
        let committed = fixture.stored();
        if replacement {
            fixture.replace(&committed);
            assert!(journal.recover().is_err());
        } else {
            journal.recover().unwrap();
            assert!(journal.intents().unwrap().is_empty());
            assert!(!journal.settle_intent(&intent).unwrap());
        }
        assert_eq!(fixture.stored(), committed);
    }
}

#[test]
fn live_unsubmitted_proof_cancels_only_the_exact_recovered_own_intent() {
    for stage in [Checkpoint::CommitObserved, Checkpoint::WriteAcknowledged] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let record = pending(HubId::new());
        let failure = fail_at(stage);
        assert!(journal.retain_intent(intent(&record)).is_err());
        drop(failure);
        // This live caller never dispatched Create; disk recovery does not prove that.
        journal.recover().unwrap();
        let retained = fixture.stored();
        let wrong = intent(&conflicting_confirmations(&record)[FIRST_INDEX]);
        assert!(journal.cancel_uncreated_intent(&wrong).is_err());
        assert_eq!(fixture.stored(), retained);
        journal.recover().unwrap();
        assert!(journal.cancel_uncreated_intent(&intent(&record)).unwrap());
        assert!(!journal.cancel_uncreated_intent(&intent(&record)).unwrap());
        assert!(journal.intents().unwrap().is_empty());
        assert!(journal.pending().unwrap().is_empty());
    }
}
