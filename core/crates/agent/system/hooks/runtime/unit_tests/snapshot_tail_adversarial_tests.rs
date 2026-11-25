use super::constants::{snapshot_fields as fields, test_values};
use super::test_support::{
    CLAUDE_VERSION, CODEX_VERSION, TURN, claude_envelope, identity, started,
};
use super::*;
use serde_json::{Value, from_value, to_value};

fn truncated_claude_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    let mut sequence = FIRST_SEQUENCE;
    reducer
        .apply(claude_envelope(
            &expected,
            sequence,
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        ))
        .unwrap();
    for _ in 0..MAX_REPLAY_WINDOW {
        sequence += FIRST_SEQUENCE;
        reducer
            .apply(claude_envelope(
                &expected,
                sequence,
                EventKind::UserPromptSubmit,
            ))
            .unwrap();
    }
    sequence += FIRST_SEQUENCE;
    let mut stop = claude_envelope(&expected, sequence, EventKind::Stop);
    stop.turn_id = Some(TURN.to_owned());
    stop.stop_hook_active = Some(true);
    reducer.apply(stop).unwrap();
    sequence += FIRST_SEQUENCE;
    reducer
        .apply(claude_envelope(&expected, sequence, EventKind::PreCompact))
        .unwrap();
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

fn restore(value: Value, expected: HookIdentity) -> Result<Reducer, SnapshotViolation> {
    Reducer::restore(
        from_value::<ReducerSnapshot>(value).unwrap(),
        expected,
        HookProviderKind::Claude,
        CLAUDE_VERSION,
    )
}

#[test]
fn truncated_history_still_enforces_static_topology() {
    let (expected, mut value) = truncated_claude_value();
    value[fields::HISTORY][fields::ENTRIES][0][fields::EVENT][fields::KIND] =
        Value::String(test_values::SUBAGENT_START_KIND.to_owned());
    assert_eq!(
        restore(value, expected).unwrap_err(),
        SnapshotViolation::InvalidHistory
    );
}

#[test]
fn truncated_history_rejects_late_startup() {
    let (expected, mut value) = truncated_claude_value();
    value[fields::HISTORY][fields::ENTRIES][0][fields::EVENT] = serde_json::json!({
        (fields::KIND): test_values::SESSION_START_KIND,
        (fields::SOURCE): test_values::STARTUP_SOURCE,
    });
    assert_eq!(
        restore(value, expected).unwrap_err(),
        SnapshotViolation::InvalidHistory
    );
}

#[test]
fn retained_compaction_and_stop_anchors_are_exact() {
    let (expected, mut phase) = truncated_claude_value();
    phase[fields::LIFECYCLE][fields::COMPACTION][fields::STARTED_AT] =
        phase[fields::HISTORY][fields::ENTRIES][0][fields::SEQUENCE].clone();
    assert_eq!(
        restore(phase, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );

    let (expected, mut stop) = truncated_claude_value();
    stop[fields::LIFECYCLE][fields::STOP_CANDIDATE][fields::EVENT_ID] =
        stop[fields::HISTORY][fields::ENTRIES][0][fields::EVENT_ID].clone();
    assert_eq!(
        restore(stop, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );

    let (expected, mut missing) = truncated_claude_value();
    missing[fields::LIFECYCLE][fields::STOP_CANDIDATE] = Value::Null;
    assert_eq!(
        restore(missing, expected).unwrap_err(),
        SnapshotViolation::InvalidState
    );
}

#[test]
fn idle_compaction_has_no_tool_or_stop_residue() {
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
    reducer
        .apply(claude_envelope(
            &expected,
            FIRST_SEQUENCE + 1,
            EventKind::PreCompact,
        ))
        .unwrap();
    reducer
        .state
        .active_tools
        .insert(test_values::IMPOSSIBLE_TOOL.to_owned());
    assert_eq!(
        reducer.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );
}

#[test]
fn revoked_exit_observation_is_revision_idempotent() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    let wrong_sequence = FIRST_SEQUENCE + 2;
    assert_eq!(
        reducer.apply(super::test_support::envelope(
            &expected,
            CODEX_VERSION,
            wrong_sequence,
            EventKind::UserPromptSubmit,
        )),
        Err(HookViolation::SequenceGap)
    );
    let revision = reducer.revision();
    let wrong_process = ProcessIdentity::new(expected.process.pid, expected.process.start_time + 1);
    assert_eq!(
        reducer.process_exit_with_identity(wrong_process),
        Ok(ProcessExitDisposition::Revoked)
    );
    assert_eq!(reducer.revision(), revision);
}
