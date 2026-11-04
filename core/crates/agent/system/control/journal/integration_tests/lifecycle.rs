use crate::{constants::*, fixture::*};
use uuid::Uuid;
use yolo_agent_control_journal::*;

#[test]
fn interrupted_new_intent_is_durable_unknown_before_any_effect_and_never_reissued_after_reopen() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let anchor = journal.anchor();
    let request = Uuid::new_v4();
    assert_eq!(journal.lookup(request, digest()).unwrap(), None);
    let reservation = reserve(&mut journal, request);
    assert_eq!(reservation.request_id(), request);
    assert_eq!(
        journal.lookup(request, digest()).unwrap(),
        Some(Receipt::Unknown)
    );
    let saved = fixture.read(&fixture.directory.join(JOURNAL_FILE));
    assert_eq!(saved[ENTRIES_FIELD][0][REQUEST_FIELD], request.to_string());
    drop(reservation);
    drop(journal);
    let mut reopened = fixture.reopen(anchor).unwrap();
    assert!(matches!(
        reopened.reserve(request, digest()).unwrap(),
        ReservationResult::Existing(Receipt::Unknown)
    ));
    assert_eq!(reopened.anchor(), anchor);
}

#[test]
fn both_definitive_receipts_survive_restart_without_reissuing_reservation_or_rewriting_history() {
    for outcome in [Completion::Delivered, Completion::NotDelivered] {
        let fixture = Fixture::new();
        let mut journal = fixture.create();
        let anchor = journal.anchor();
        let request = Uuid::new_v4();
        let reservation = reserve(&mut journal, request);
        let expected = Receipt::from(outcome);
        assert_eq!(journal.complete(reservation, outcome).unwrap(), expected);
        let path = fixture.directory.join(JOURNAL_FILE);
        let before = fixture.version(&path);
        drop(journal);
        let mut reopened = fixture.reopen(anchor).unwrap();
        assert_eq!(reopened.lookup(request, digest()).unwrap(), Some(expected));
        assert!(
            matches!(reopened.reserve(request, digest()).unwrap(), ReservationResult::Existing(actual) if actual == expected)
        );
        assert_eq!(fixture.version(&path), before);
    }
}

#[test]
fn request_collision_and_invalid_id_do_not_poison_valid_reservation() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let request = Uuid::new_v4();
    let token = reserve(&mut journal, request);
    let before = fixture.version(&fixture.directory.join(JOURNAL_FILE));
    assert!(matches!(
        journal.reserve(request, other_digest()),
        Err(JournalError::Conflict)
    ));
    assert_eq!(
        journal.lookup(request, other_digest()),
        Err(JournalError::Conflict)
    );
    assert!(matches!(
        journal.reserve(Uuid::nil(), digest()),
        Err(JournalError::InvalidRequest)
    ));
    assert_eq!(
        journal.lookup(Uuid::nil(), digest()),
        Err(JournalError::InvalidRequest)
    );
    assert_eq!(
        fixture.version(&fixture.directory.join(JOURNAL_FILE)),
        before
    );
    assert_eq!(
        journal.complete(token, Completion::Delivered).unwrap(),
        Receipt::Delivered
    );
    let next = reserve(&mut journal, Uuid::new_v4());
    journal.complete(next, Completion::NotDelivered).unwrap();
}

#[test]
fn a_foreign_journal_cannot_complete_reservation_and_neither_owner_is_poisoned() {
    let first = Fixture::new();
    let second = Fixture::new();
    let mut original = first.create();
    let mut other = second.create();
    let request = Uuid::new_v4();
    let token = reserve(&mut original, request);
    assert_eq!(
        other.complete(token, Completion::Delivered),
        Err(JournalError::ForeignReservation)
    );
    assert_eq!(
        original.lookup(request, digest()).unwrap(),
        Some(Receipt::Unknown)
    );
    assert_eq!(other.lookup(request, digest()).unwrap(), None);
    let own = reserve(&mut other, request);
    other.complete(own, Completion::NotDelivered).unwrap();
    let next = reserve(&mut original, Uuid::new_v4());
    original.complete(next, Completion::Delivered).unwrap();
}

#[test]
fn reservation_cannot_cross_journal_reopen_even_with_identical_scope_and_anchor() {
    let fixture = Fixture::new();
    let mut original = fixture.create();
    let anchor = original.anchor();
    let request = Uuid::new_v4();
    let old = reserve(&mut original, request);
    drop(original);
    let mut reopened = fixture.reopen(anchor).unwrap();
    assert_eq!(
        reopened.complete(old, Completion::NotDelivered),
        Err(JournalError::ForeignReservation)
    );
    assert!(matches!(
        reopened.reserve(request, digest()).unwrap(),
        ReservationResult::Existing(Receipt::Unknown)
    ));
    let next = reserve(&mut reopened, Uuid::new_v4());
    reopened.complete(next, Completion::Delivered).unwrap();
}
