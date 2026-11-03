use super::{constants::*, fixture::*};
use crate::{Completion, JOURNAL_FILE, Journal, JournalError, JournalScope};
use std::error::Error;
use uuid::Uuid;

pub(super) fn assert_private(error: JournalError, fixture: &Fixture) {
    let public = [
        error.to_string(),
        format!("{error:?}"),
        format!("{error:#}"),
    ];
    for message in public {
        assert!(!message.is_empty());
        assert!(!message.contains(fixture.directory.to_str().unwrap()));
        assert!(!message.contains(PRIVATE_RECORD_CONTENT));
    }
    assert!(error.source().is_none());
}

#[test]
fn actual_business_rejections_expose_only_public_diagnostics() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    let anchor = journal.anchor();
    let invalid = journal.lookup(Uuid::nil(), digest()).unwrap_err();
    assert_eq!(invalid, JournalError::InvalidRequest);
    assert_private(invalid, &fixture);
    let busy = fixture.reopen(anchor).err().expect(EXPECTED_REJECTION);
    assert_eq!(busy, JournalError::Busy);
    assert_private(busy, &fixture);
    let request = Uuid::new_v4();
    let token = reserve(&mut journal, request);
    let conflict = journal.lookup(request, other_digest()).unwrap_err();
    assert_eq!(conflict, JournalError::Conflict);
    assert_private(conflict, &fixture);
    let other = Fixture::new();
    let mut stranger = other.create();
    let foreign = stranger
        .complete(token, Completion::NotDelivered)
        .unwrap_err();
    assert_eq!(foreign, JournalError::ForeignReservation);
    assert_private(foreign, &fixture);
    assert_private(foreign, &other);
    drop(journal);
    let existing = Journal::create_new(fixture.retained(), fixture.scope)
        .err()
        .expect(EXPECTED_REJECTION);
    assert_eq!(existing, JournalError::AlreadyExists);
    assert_private(existing, &fixture);
    let wrong_scope = JournalScope::new(Uuid::new_v4(), fixture.scope.runtime_epoch()).unwrap();
    let mismatch = Journal::open_existing(fixture.retained(), wrong_scope, anchor)
        .err()
        .expect(EXPECTED_REJECTION);
    assert_eq!(mismatch, JournalError::ScopeMismatch);
    assert_private(mismatch, &fixture);
}

#[test]
fn malformed_private_record_contents_are_not_exposed_through_errors_or_source_chains() {
    let fixture = Fixture::new();
    let journal = fixture.create();
    let path = fixture.directory.join(JOURNAL_FILE);
    let mut record = fixture.read(&path);
    record[SCHEMA_FIELD] = serde_json::Value::String(PRIVATE_RECORD_CONTENT.into());
    fixture.write(&path, &record);
    let invalid = journal.verify().unwrap_err();
    assert_eq!(invalid, JournalError::Unavailable);
    assert_private(invalid, &fixture);
    let poisoned = journal.verify().unwrap_err();
    assert_eq!(poisoned, JournalError::Poisoned);
    assert_private(poisoned, &fixture);
}

#[test]
fn failed_durable_write_diagnostics_do_not_forward_private_path_or_nested_storage_errors() {
    let fixture = Fixture::new();
    let mut journal = fixture.create();
    yolo_paths::test_support::fail_next_directory_sync();
    let failure = journal.reserve(Uuid::new_v4(), digest()).unwrap_err();
    assert_eq!(failure, JournalError::OutcomeUnknown);
    assert_private(failure, &fixture);
    assert_private(journal.verify().unwrap_err(), &fixture);
}
