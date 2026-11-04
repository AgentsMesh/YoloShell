use super::constants::{snapshot_fields as fields, snapshot_indexes as indexes, test_values};
use super::test_support::{CODEX_VERSION, identity, started};
use super::*;
use serde_json::{Map, Value, from_value, to_value};

const ARBITRARY_REVISION: u64 = 42;

fn started_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

fn restore(value: Value, expected: HookIdentity) -> Result<Reducer, SnapshotViolation> {
    let snapshot = from_value::<ReducerSnapshot>(value).unwrap();
    Reducer::restore(snapshot, expected, HookProviderKind::Codex, CODEX_VERSION)
}

fn extra() -> Value {
    Value::String(test_values::FORBIDDEN.to_owned())
}

#[test]
fn every_nested_document_rejects_unknown_fields() {
    let (_, original) = started_value();
    let history = fields::HISTORY;
    let entries = fields::ENTRIES;
    let first = indexes::FIRST;
    let paths: &[&[&str]] = &[
        &[],
        &[fields::BINDING],
        &[fields::BINDING, fields::IDENTITY],
        &[fields::BINDING, fields::IDENTITY, fields::PROCESS],
        &[fields::BINDING, fields::PROVIDER],
        &[fields::BINDING, fields::PROVIDER_VERSION],
        &[fields::LIFECYCLE],
        &[fields::HISTORY],
        &[history, entries, first],
        &[history, entries, first, fields::IDENTITY],
        &[history, entries, first, fields::IDENTITY, fields::PROCESS],
        &[history, entries, first, fields::PROVIDER],
        &[history, entries, first, fields::PROVIDER_VERSION],
        &[history, entries, first, fields::EVENT],
    ];
    for path in paths {
        let mut tampered = original.clone();
        object_at(&mut tampered, path).insert(fields::UNKNOWN.to_owned(), extra());
        assert!(from_value::<ReducerSnapshot>(tampered).is_err(), "{path:?}");
    }
}

#[test]
fn history_limit_continuity_tail_and_event_ids_are_strict() {
    let (expected, original) = started_value();
    let entry = original[fields::HISTORY][fields::ENTRIES][0].clone();
    let mut oversized = original.clone();
    oversized[fields::HISTORY][fields::ENTRIES] =
        Value::Array(std::iter::repeat_n(entry, MAX_REPLAY_WINDOW + 1).collect());
    assert_eq!(
        restore(oversized, expected.clone()).unwrap_err(),
        SnapshotViolation::HistoryTooLarge
    );

    let mut wrong_tail = original.clone();
    wrong_tail[fields::LIFECYCLE][fields::LAST_SEQUENCE] = Value::from(FIRST_SEQUENCE + 1);
    assert_eq!(
        restore(wrong_tail, expected.clone()).unwrap_err(),
        SnapshotViolation::InvalidHistory
    );

    let serialized = serde_json::to_string(&original).unwrap();
    assert!(!serialized.contains(fields::EVENT_IDS));
    let mut wrong_event_id = original;
    wrong_event_id[fields::HISTORY][fields::ENTRIES][0][fields::EVENT_ID] =
        Value::String(uuid::Uuid::nil().to_string());
    assert_eq!(
        restore(wrong_event_id, expected).unwrap_err(),
        SnapshotViolation::InvalidHistory
    );
}

#[test]
fn semantic_state_tampering_is_not_restorable() {
    let (expected, mut value) = started_value();
    value[fields::LIFECYCLE][fields::TURN] = Value::String(test_values::RUNNING.to_owned());
    assert_eq!(
        restore(value, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );
}

#[test]
fn zero_exhausted_and_unbound_revisions_are_rejected() {
    let (expected, mut zero) = started_value();
    zero[fields::AUTHORITY_REVISION] = Value::from(0_u64);
    assert_eq!(
        restore(zero, expected.clone()).unwrap_err(),
        SnapshotViolation::InvalidRevision
    );

    let (expected, mut exhausted) = started_value();
    exhausted[fields::AUTHORITY_REVISION] = Value::from(u64::MAX);
    assert_eq!(
        restore(exhausted, expected).unwrap_err(),
        SnapshotViolation::InvalidRevision
    );

    let (expected, mut unbound) = started_value();
    unbound[fields::AUTHORITY_REVISION] = Value::from(u64::MAX - FIRST_SEQUENCE);
    assert_eq!(
        restore(unbound, expected).unwrap_err(),
        SnapshotViolation::InvalidRevision
    );

    let (expected, mut arbitrary) = started_value();
    arbitrary[fields::AUTHORITY_REVISION] = Value::from(ARBITRARY_REVISION);
    assert_eq!(
        restore(arbitrary, expected).unwrap_err(),
        SnapshotViolation::InvalidRevision
    );
}

#[test]
fn terminal_revision_candidates_are_lifecycle_bound() {
    let expected = identity();
    let mut exited = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut exited, &expected);
    exited.process_exit();
    let exited_value = to_value(exited.snapshot().unwrap()).unwrap();
    assert!(restore(exited_value.clone(), expected.clone()).is_ok());
    for revision in [
        exited.revision() - FIRST_SEQUENCE,
        exited.revision() + FIRST_SEQUENCE,
    ] {
        let mut invalid = exited_value.clone();
        invalid[fields::AUTHORITY_REVISION] = Value::from(revision);
        assert_eq!(
            restore(invalid, expected.clone()).unwrap_err(),
            SnapshotViolation::InvalidRevision
        );
    }

    let mut revoked = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut revoked, &expected);
    let gap = super::test_support::envelope(
        &expected,
        CODEX_VERSION,
        FIRST_SEQUENCE + 2,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(revoked.apply(gap), Err(HookViolation::SequenceGap));
    let revoked_value = to_value(revoked.snapshot().unwrap()).unwrap();
    assert!(restore(revoked_value.clone(), expected.clone()).is_ok());
    for revision in [
        revoked.revision() - FIRST_SEQUENCE,
        revoked.revision() + FIRST_SEQUENCE + FIRST_SEQUENCE,
    ] {
        let mut invalid = revoked_value.clone();
        invalid[fields::AUTHORITY_REVISION] = Value::from(revision);
        assert_eq!(
            restore(invalid, expected.clone()).unwrap_err(),
            SnapshotViolation::InvalidRevision
        );
    }
}

fn object_at<'a>(value: &'a mut Value, path: &[&str]) -> &'a mut Map<String, Value> {
    let mut current = value;
    for segment in path {
        current = if let Ok(index) = segment.parse::<usize>() {
            &mut current.as_array_mut().unwrap()[index]
        } else {
            &mut current[*segment]
        };
    }
    current.as_object_mut().unwrap()
}
