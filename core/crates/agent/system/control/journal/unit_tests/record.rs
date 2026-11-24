use super::{constants::*, fixture::*};
use crate::{
    MAX_RECEIPTS, Receipt,
    record::{Entry, Header, Record},
};
use uuid::Uuid;

#[test]
fn receipt_record_rejects_foreign_scope_duplicate_nil_ids_and_excess_history() {
    let fixture = Fixture::new();
    let journal = fixture.create();
    let anchor = journal.anchor();
    let mut record = Record::empty(anchor);
    record.entries.push(Entry {
        request_id: Uuid::new_v4(),
        digest: digest(),
        receipt: Receipt::Unknown,
    });
    record.validate(anchor).unwrap();
    let mut foreign = record.clone();
    foreign.anchor.journal_id = Uuid::new_v4();
    assert!(foreign.validate(anchor).is_err());
    let mut unknown = record.clone();
    unknown.schema += SCHEMA_STEP;
    assert!(unknown.validate(anchor).is_err());
    let mut nil = record.clone();
    nil.entries[0].request_id = Uuid::from_u128(NIL_SEQUENCE);
    assert!(nil.validate(anchor).is_err());
    let mut duplicate = record.clone();
    duplicate.entries.push(duplicate.entries[0]);
    assert!(duplicate.validate(anchor).is_err());
    record.entries = (FIRST_REQUEST..=u128::try_from(MAX_RECEIPTS).unwrap() + FIRST_REQUEST)
        .map(|id| Entry {
            request_id: Uuid::from_u128(id),
            digest: digest(),
            receipt: Receipt::Unknown,
        })
        .collect();
    assert!(record.validate(anchor).is_err());
}

#[test]
fn immutable_header_cannot_switch_schema_scope_or_initialization_identity() {
    let fixture = Fixture::new();
    let journal = fixture.create();
    let anchor = journal.anchor();
    let mut header = journal.header;
    header.validate(anchor).unwrap();
    header.schema += SCHEMA_STEP;
    assert!(header.validate(anchor).is_err());
    let other = Header::new(fixture.scope, anchor.directory);
    assert!(other.validate(anchor).is_err());
}
