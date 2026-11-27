use super::constants::snapshot_fields as fields;
use super::test_support::{
    CLAUDE_VERSION, CODEX_VERSION, claude_envelope, envelope, identity, started,
};
use super::*;
use serde_json::{Value, from_value, to_value};

const REJECTED_EVENT_SEQUENCE: u64 = FIRST_SEQUENCE + 2;
const TAMPERED_PROOF_SEQUENCE: u64 = REJECTED_EVENT_SEQUENCE + 1;
const PROCESS_START_TIME_DELTA: u64 = 1;

fn restore(value: Value, expected: HookIdentity) -> Result<Reducer, SnapshotViolation> {
    Reducer::restore(
        from_value::<ReducerSnapshot>(value).unwrap(),
        expected,
        HookProviderKind::Codex,
        CODEX_VERSION,
    )
}

fn hook_revoked_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let rejected = envelope(
        &expected,
        CODEX_VERSION,
        REJECTED_EVENT_SEQUENCE,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(reducer.apply(rejected), Err(HookViolation::SequenceGap));
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

fn process_revoked_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let observed = ProcessIdentity::new(
        expected.process.pid,
        expected
            .process
            .start_time
            .checked_add(PROCESS_START_TIME_DELTA)
            .unwrap(),
    );
    assert_eq!(
        reducer.process_exit_with_identity(observed),
        Err(HookViolation::ProcessMismatch)
    );
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

fn revocation(value: &mut Value) -> &mut Value {
    &mut value[fields::LIFECYCLE][fields::REVOCATION]
}

fn assert_hook_tamper(mutator: impl FnOnce(&mut Value)) {
    let (expected, mut value) = hook_revoked_value();
    mutator(revocation(&mut value));
    assert_eq!(
        restore(value, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );
}

fn truncated_reducer() -> (HookIdentity, Reducer) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    reducer
        .apply(claude_envelope(
            &expected,
            FIRST_SEQUENCE,
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        ))
        .unwrap();
    for offset in 0..MAX_REPLAY_WINDOW {
        let sequence = FIRST_SEQUENCE + 1 + u64::try_from(offset).unwrap();
        reducer
            .apply(claude_envelope(
                &expected,
                sequence,
                EventKind::UserPromptSubmit,
            ))
            .unwrap();
    }
    assert_ne!(
        reducer.state().history.keys().next().copied(),
        Some(EventSequence::FIRST)
    );
    (expected, reducer)
}

#[test]
fn truncated_history_round_trips_hook_event_revocation() {
    let (expected, mut reducer) = truncated_reducer();
    let rejected_sequence = reducer.state().last_sequence.0 + 2;
    let rejected = claude_envelope(&expected, rejected_sequence, EventKind::UserPromptSubmit);

    assert_eq!(reducer.apply(rejected), Err(HookViolation::SequenceGap));
    let encoded = to_value(reducer.snapshot().unwrap()).unwrap();
    let restored = Reducer::restore(
        from_value(encoded).unwrap(),
        expected,
        HookProviderKind::Claude,
        CLAUDE_VERSION,
    )
    .unwrap();
    assert_eq!(restored.state(), reducer.state());
    assert_eq!(restored.revision(), reducer.revision());
}

#[test]
fn truncated_history_round_trips_process_revocation() {
    let (expected, mut reducer) = truncated_reducer();
    let observed = ProcessIdentity::new(
        expected.process.pid,
        expected
            .process
            .start_time
            .checked_add(PROCESS_START_TIME_DELTA)
            .unwrap(),
    );

    assert_eq!(
        reducer.process_exit_with_identity(observed),
        Err(HookViolation::ProcessMismatch)
    );
    let encoded = to_value(reducer.snapshot().unwrap()).unwrap();
    let restored = Reducer::restore(
        from_value(encoded).unwrap(),
        expected,
        HookProviderKind::Claude,
        CLAUDE_VERSION,
    )
    .unwrap();
    assert_eq!(restored.state(), reducer.state());
    assert_eq!(restored.revision(), reducer.revision());
}

#[test]
fn revocation_variants_reject_unknown_fields() {
    for (_, mut value) in [hook_revoked_value(), process_revoked_value()] {
        revocation(&mut value)
            .as_object_mut()
            .unwrap()
            .insert(fields::UNKNOWN.to_owned(), Value::Bool(true));
        assert!(from_value::<ReducerSnapshot>(value).is_err());
    }
}

#[test]
fn hook_revocation_proof_is_exactly_bound_to_rejected_envelope() {
    assert_hook_tamper(|proof| {
        proof[fields::EVENT_ID] = to_value(uuid::Uuid::new_v4()).unwrap();
    });
    assert_hook_tamper(|proof| {
        proof[fields::SEQUENCE] = Value::from(TAMPERED_PROOF_SEQUENCE);
    });
    assert_hook_tamper(|proof| {
        proof[fields::VIOLATION] = to_value(HookViolation::ProcessMismatch).unwrap();
    });
    let (_, other) = hook_revoked_value();
    let other_digest =
        other[fields::LIFECYCLE][fields::REVOCATION][fields::ENVELOPE_SHA256].clone();
    assert_hook_tamper(|proof| {
        proof[fields::ENVELOPE_SHA256] = other_digest;
    });
}

#[test]
fn process_revocation_requires_a_distinct_valid_observed_identity() {
    let (expected, mut same) = process_revoked_value();
    revocation(&mut same)[fields::PID] = Value::from(expected.process.pid);
    revocation(&mut same)[fields::START_TIME] = Value::from(expected.process.start_time);
    assert_eq!(
        restore(same, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );

    let (expected, mut zero_pid) = process_revoked_value();
    revocation(&mut zero_pid)[fields::PID] = Value::from(u32::default());
    assert_eq!(
        restore(zero_pid, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );

    let (expected, mut zero_start) = process_revoked_value();
    revocation(&mut zero_start)[fields::START_TIME] = Value::from(u64::default());
    assert_eq!(
        restore(zero_start, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );
}
