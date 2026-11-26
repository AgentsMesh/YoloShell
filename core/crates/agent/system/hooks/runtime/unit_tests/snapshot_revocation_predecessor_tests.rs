use super::constants::snapshot_fields as fields;
use super::test_support::{CLAUDE_VERSION, FIRST_POST_START_SEQUENCE, claude_envelope, identity};
use super::*;
use serde_json::{Value, from_value, to_value};

const REJECTION_SEQUENCE_GAP: u64 = 2;
const FILLER_BYTE: char = 'x';

#[test]
fn maximal_truncated_revocation_snapshot_fits_the_durable_budget() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    apply_start(&mut reducer, &expected);
    for offset in 0..MAX_REPLAY_WINDOW {
        let sequence = FIRST_POST_START_SEQUENCE + u64::try_from(offset).unwrap();
        reducer.apply(maximal_event(&expected, sequence)).unwrap();
    }
    let rejected_sequence = reducer.state().last_sequence.0 + REJECTION_SEQUENCE_GAP;
    assert_eq!(
        reducer.apply(maximal_event(&expected, rejected_sequence)),
        Err(HookViolation::SequenceGap)
    );

    let encoded = serde_json::to_vec(&reducer.snapshot().unwrap()).unwrap();
    assert!(encoded.len() <= yolo_storage::DEFAULT_JSON_MAX_BYTES);
}

#[test]
fn predecessor_revision_tamper_is_rejected() {
    assert_predecessor_tamper(|predecessor| {
        let revision = predecessor[fields::AUTHORITY_REVISION].as_u64().unwrap();
        predecessor[fields::AUTHORITY_REVISION] = Value::from(revision + AUTHORITY_REVISION_STEP);
    });
}

#[test]
fn predecessor_binding_tamper_is_rejected() {
    assert_predecessor_tamper(|predecessor| {
        predecessor[fields::BINDING][fields::IDENTITY][fields::RUNTIME_EPOCH] =
            to_value(uuid::Uuid::new_v4()).unwrap();
    });
}

#[test]
fn predecessor_history_tamper_is_rejected() {
    assert_predecessor_tamper(|predecessor| {
        let first = predecessor[fields::HISTORY][fields::ENTRIES]
            .as_array_mut()
            .unwrap()
            .first_mut()
            .unwrap();
        first[fields::EVENT_ID] = to_value(uuid::Uuid::new_v4()).unwrap();
    });
}

#[test]
fn revoked_predecessor_is_rejected() {
    let (expected, mut value) = revoked_value();
    let nested = value.clone();
    *predecessor(&mut value) = nested;
    assert!(restore(value, expected).is_err());
}

#[test]
fn missing_predecessor_is_rejected_by_the_wire_schema() {
    let (_, mut value) = revoked_value();
    revocation(&mut value)
        .as_object_mut()
        .unwrap()
        .remove(fields::PREDECESSOR);
    assert!(from_value::<ReducerSnapshot>(value).is_err());
}

fn revoked_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    apply_start(&mut reducer, &expected);
    let rejected = claude_envelope(
        &expected,
        FIRST_SEQUENCE + REJECTION_SEQUENCE_GAP,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(reducer.apply(rejected), Err(HookViolation::SequenceGap));
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

fn assert_predecessor_tamper(mutator: impl FnOnce(&mut Value)) {
    let (expected, mut value) = revoked_value();
    mutator(predecessor(&mut value));
    assert!(restore(value, expected).is_err());
}

fn restore(value: Value, expected: HookIdentity) -> Result<Reducer, SnapshotViolation> {
    Reducer::restore(
        from_value(value).unwrap(),
        expected,
        HookProviderKind::Claude,
        CLAUDE_VERSION,
    )
}

fn predecessor(value: &mut Value) -> &mut Value {
    &mut revocation(value)[fields::PREDECESSOR]
}

fn revocation(value: &mut Value) -> &mut Value {
    &mut value[fields::LIFECYCLE][fields::REVOCATION]
}

fn apply_start(reducer: &mut Reducer, expected: &HookIdentity) {
    let start = claude_envelope(
        expected,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    reducer.apply(start).unwrap();
}

fn maximal_event(expected: &HookIdentity, sequence: u64) -> HookEnvelope {
    let mut event = claude_envelope(
        expected,
        sequence,
        EventKind::Notification {
            kind: NotificationKind::Other(String::new()),
        },
    );
    let mut measured = event.clone();
    measured.sequence = EventSequence(u64::MAX);
    let baseline = serde_json::to_vec(&measured).unwrap().len();
    let filler_len = super::constants::MAX_HOOK_ENVELOPE_BYTES - baseline;
    event.event = EventKind::Notification {
        kind: NotificationKind::Other(std::iter::repeat_n(FILLER_BYTE, filler_len).collect()),
    };
    assert!(event.has_bounded_storage_encoding());
    event
}
