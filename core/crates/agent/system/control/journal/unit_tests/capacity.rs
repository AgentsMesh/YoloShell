use super::{constants::*, fixture::*};
use crate::{Completion, JournalError, MAX_RECEIPTS, Receipt, ReservationResult, record::Entry};
use uuid::Uuid;

#[test]
fn full_history_never_evicts_receipts_and_capacity_rejection_allows_existing_completion() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let pending_request = Uuid::new_v4();
    let pending = reserve(&mut journal, pending_request);
    // Seed a valid prior history directly instead of thousands of redundant fsyncs.
    let mut full = journal.record.clone();
    let first = Uuid::from_u128(FIRST_REQUEST);
    for index in FIRST_REQUEST..u128::try_from(MAX_RECEIPTS).unwrap() {
        full.entries.push(Entry {
            request_id: Uuid::from_u128(index),
            digest: digest(),
            receipt: Receipt::Delivered,
        });
    }
    full.validate(journal.anchor()).unwrap();
    assert!(
        u64::try_from(serde_json::to_vec_pretty(&full).unwrap().len()).unwrap()
            <= yolo_paths::constants::MAX_DURABLE_JSON_BYTES
    );
    journal.commit(full).unwrap();
    let exhausted = journal.reserve(Uuid::new_v4(), digest()).unwrap_err();
    assert_eq!(exhausted, JournalError::Capacity);
    super::diagnostics::assert_private(exhausted, &fixture);
    assert!(matches!(
        journal.reserve(first, digest()).unwrap(),
        ReservationResult::Existing(Receipt::Delivered)
    ));
    assert!(matches!(
        journal.reserve(first, other_digest()),
        Err(JournalError::Conflict)
    ));
    assert_eq!(
        journal.complete(pending, Completion::NotDelivered).unwrap(),
        Receipt::NotDelivered
    );
    assert_eq!(journal.record.entries.len(), MAX_RECEIPTS);
    let anchor = journal.anchor();
    drop(journal);
    let mut reopened = fixture.reopen(anchor).unwrap();
    assert_eq!(
        reopened.lookup(first, digest()).unwrap(),
        Some(Receipt::Delivered)
    );
    assert_eq!(
        reopened.lookup(pending_request, digest()).unwrap(),
        Some(Receipt::NotDelivered)
    );
    assert!(matches!(
        reopened.reserve(Uuid::new_v4(), digest()),
        Err(JournalError::Capacity)
    ));
    reopened.verify().unwrap();
}
