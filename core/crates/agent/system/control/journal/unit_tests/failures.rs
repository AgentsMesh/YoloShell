use super::fixture::*;
use crate::{Completion, JournalError, Receipt, ReservationResult};
use uuid::Uuid;

#[test]
fn failed_intent_fsync_never_returns_new_authority_and_poison_is_sticky() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let anchor = journal.anchor();
    let request = Uuid::new_v4();
    yolo_paths::test_support::fail_next_directory_sync();
    assert!(matches!(
        journal.reserve(request, digest()),
        Err(JournalError::OutcomeUnknown)
    ));
    assert_eq!(journal.verify(), Err(JournalError::Poisoned));
    assert!(matches!(
        journal.reserve(request, digest()),
        Err(JournalError::Poisoned)
    ));
    drop(journal);
    let mut reopened = fixture.reopen(anchor).unwrap();
    // Failure before publishing the intent leaves it absent; failure after publishing
    // leaves Unknown. No New was returned by the interrupted attempt in either case.
    match reopened.reserve(request, digest()).unwrap() {
        ReservationResult::Existing(receipt) => assert_eq!(receipt, Receipt::Unknown),
        ReservationResult::New(reservation) => {
            assert_eq!(
                reopened
                    .complete(reservation, Completion::NotDelivered)
                    .unwrap(),
                Receipt::NotDelivered
            );
        }
    }
}

#[test]
fn failed_completion_never_reissues_effect_after_the_owner_restarts() {
    for completion in [Completion::Delivered, Completion::NotDelivered] {
        let fixture = Fixture::new();
        let mut journal = fixture.create();
        let anchor = journal.anchor();
        let request = Uuid::new_v4();
        let token = reserve(&mut journal, request);
        yolo_paths::test_support::fail_next_directory_sync();
        assert_eq!(
            journal.complete(token, completion),
            Err(JournalError::OutcomeUnknown)
        );
        assert_eq!(journal.verify(), Err(JournalError::Poisoned));
        drop(journal);
        let mut reopened = fixture.reopen(anchor).unwrap();
        match reopened.reserve(request, digest()).unwrap() {
            ReservationResult::Existing(receipt) => {
                assert!(receipt == Receipt::Unknown || receipt == Receipt::from(completion));
            }
            ReservationResult::New(_) => panic!("completion failure reissued the effect"),
        }
    }
}

#[test]
fn disappearing_receipts_before_completion_poison_owner_without_recreating_history() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let token = reserve(&mut journal, Uuid::new_v4());
    std::fs::remove_file(&journal.record_path).unwrap();
    assert_eq!(
        journal.complete(token, Completion::Delivered),
        Err(JournalError::Unavailable)
    );
    assert!(!journal.record_path.exists());
    assert!(matches!(
        journal.reserve(Uuid::new_v4(), digest()),
        Err(JournalError::Poisoned)
    ));
}
