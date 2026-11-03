use crate::{constants::*, fixture::*};
use uuid::Uuid;
use yolo_agent_control_journal::*;

#[test]
fn malformed_duplicate_or_foreign_scope_receipt_history_cannot_be_reopened() {
    let mutations: &[fn(&mut serde_json::Value)] = &[
        |value| value[SCHEMA_FIELD] = serde_json::json!(u16::MAX),
        |value| {
            let entry = value[ENTRIES_FIELD][0].clone();
            value[ENTRIES_FIELD].as_array_mut().unwrap().push(entry);
        },
        |value| value[ANCHOR_FIELD][JOURNAL_ID_FIELD] = serde_json::json!(Uuid::new_v4()),
        |value| value[ENTRIES_FIELD][0][REQUEST_FIELD] = serde_json::json!(Uuid::nil()),
        |value| {
            value[ENTRIES_FIELD][0][REQUEST_FIELD] = serde_json::json!(CANONICAL_ID.to_uppercase())
        },
        |value| value[UNKNOWN_FIELD] = serde_json::Value::Bool(true),
    ];
    for mutate in mutations {
        let fixture = Fixture::new();
        let mut journal = fixture.create();
        let anchor = journal.anchor();
        drop(reserve(&mut journal, Uuid::new_v4()));
        drop(journal);
        let path = fixture.directory.join(JOURNAL_FILE);
        let mut saved = fixture.read(&path);
        mutate(&mut saved);
        fixture.write(&path, &saved);
        let version = fixture.version(&path);
        assert!(fixture.reopen(anchor).is_err());
        assert_eq!(fixture.version(&path), version);
    }
}

#[test]
fn matching_directory_is_insufficient_when_session_or_runtime_epoch_changes() {
    let fixture = Fixture::new();
    let journal = fixture.create();
    let anchor = journal.anchor();
    drop(journal);
    for scope in [
        JournalScope::new(Uuid::new_v4(), fixture.scope.runtime_epoch()).unwrap(),
        JournalScope::new(fixture.scope.terminal_id(), Uuid::new_v4()).unwrap(),
    ] {
        assert!(matches!(
            Journal::open_existing(fixture.retained(), scope, anchor),
            Err(JournalError::ScopeMismatch)
        ));
    }
    fixture.reopen(anchor).unwrap().verify().unwrap();
}
