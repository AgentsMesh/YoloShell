use super::{constants::*, fixture::*};
use crate::{HEADER_FILE, Journal, JournalAnchor, JournalError, JournalScope};
use uuid::Uuid;

#[test]
fn scope_requires_both_non_nil_ids_and_wire_spelling_is_canonical_and_closed() {
    for (session, epoch) in [(Uuid::nil(), Uuid::new_v4()), (Uuid::new_v4(), Uuid::nil())] {
        assert_eq!(
            JournalScope::new(session, epoch),
            Err(JournalError::InvalidRequest)
        );
    }
    let scope = JournalScope::new(Uuid::new_v4(), Uuid::new_v4()).unwrap();
    let encoded = serde_json::to_value(scope).unwrap();
    assert_eq!(
        serde_json::from_value::<JournalScope>(encoded.clone()).unwrap(),
        scope
    );
    for field in [TERMINAL_FIELD, EPOCH_FIELD] {
        for invalid in [Uuid::nil().to_string(), CANONICAL_ID.to_uppercase()] {
            let mut value = encoded.clone();
            value[field] = serde_json::Value::String(invalid);
            assert!(serde_json::from_value::<JournalScope>(value).is_err());
        }
    }
    let mut unknown = encoded;
    unknown[UNKNOWN_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<JournalScope>(unknown).is_err());
}

#[test]
fn external_anchor_round_trips_exact_header_identity_and_rejects_nil_or_noncanonical_id() {
    let fixture = Fixture::new();
    let journal = fixture.create();
    let anchor = journal.anchor();
    assert_eq!(anchor.scope(), fixture.scope);
    assert!(!anchor.journal_id().is_nil());
    let encoded = serde_json::to_value(anchor).unwrap();
    assert_eq!(
        serde_json::from_value::<JournalAnchor>(encoded.clone()).unwrap(),
        anchor
    );
    for invalid in [Uuid::nil().to_string(), CANONICAL_ID.to_uppercase()] {
        let mut value = encoded.clone();
        value[JOURNAL_ID_FIELD] = serde_json::Value::String(invalid);
        assert!(serde_json::from_value::<JournalAnchor>(value).is_err());
    }
    let mut unknown = encoded;
    unknown[UNKNOWN_FIELD] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<JournalAnchor>(unknown).is_err());
}

#[test]
fn altered_external_scope_with_original_header_identity_cannot_rebind_existing_journal() {
    let fixture = Fixture::new();
    let journal = fixture.create();
    let anchor = journal.anchor();
    let header = fixture.directory.join(HEADER_FILE);
    let original = fixture.version(&header);
    drop(journal);
    for scope in [
        JournalScope::new(Uuid::new_v4(), fixture.scope.runtime_epoch()).unwrap(),
        JournalScope::new(fixture.scope.terminal_id(), Uuid::new_v4()).unwrap(),
    ] {
        let mut changed = anchor;
        changed.scope = scope;
        assert_eq!(changed.header_identity, anchor.header_identity);
        assert!(matches!(
            Journal::open_existing(fixture.retained(), scope, changed),
            Err(JournalError::ScopeMismatch)
        ));
        assert_eq!(fixture.version(&header), original);
    }
    fixture.reopen(anchor).unwrap().verify().unwrap();
}
