use super::constants::{snapshot_fields as fields, test_values};
use super::test_support::{
    CHILD, CLAUDE_VERSION, CODEX_VERSION, TURN, claude_envelope, envelope, identity, started,
};
use super::*;
use serde_json::{Value, from_value, to_value};

const CHILD_START_SEQUENCE: u64 = FIRST_SEQUENCE + 3;
const CHILD_COMPACT_SEQUENCE: u64 = FIRST_SEQUENCE + 4;
fn restore(
    value: Value,
    expected: HookIdentity,
    provider: HookProviderKind,
    version: ProviderVersion,
) -> Result<Reducer, SnapshotViolation> {
    Reducer::restore(
        from_value::<ReducerSnapshot>(value).unwrap(),
        expected,
        provider,
        version,
    )
}

pub(super) fn rich_claude_value() -> (HookIdentity, Value) {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    let events = [
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
        EventKind::Stop,
        EventKind::PreCompact,
        EventKind::SubagentStart,
        EventKind::PreCompact,
        EventKind::Notification {
            kind: NotificationKind::AgentNeedsInput,
        },
    ];
    for (offset, event) in events.into_iter().enumerate() {
        let sequence = FIRST_SEQUENCE + u64::try_from(offset).unwrap();
        let mut envelope = claude_envelope(&expected, sequence, event);
        if sequence == FIRST_SEQUENCE + 1 {
            envelope.turn_id = Some(TURN.to_owned());
        }
        if matches!(sequence, CHILD_START_SEQUENCE | CHILD_COMPACT_SEQUENCE) {
            envelope.agent_id = Some(CHILD.to_owned());
        }
        reducer.apply(envelope).unwrap();
    }
    (expected, to_value(reducer.snapshot().unwrap()).unwrap())
}

#[test]
fn retained_history_cannot_be_shortened_to_bypass_replay() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    reducer
        .apply(envelope(
            &expected,
            CODEX_VERSION,
            FIRST_SEQUENCE + 1,
            EventKind::UserPromptSubmit,
        ))
        .unwrap();
    let mut value = to_value(reducer.snapshot().unwrap()).unwrap();
    value[fields::HISTORY][fields::ENTRIES]
        .as_array_mut()
        .unwrap()
        .remove(0);
    assert_eq!(
        restore(value, expected, HookProviderKind::Codex, CODEX_VERSION).unwrap_err(),
        SnapshotViolation::InvalidHistory
    );
}

#[test]
fn inactive_child_and_idle_root_are_canonical() {
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
    let mut child = claude_envelope(&expected, FIRST_SEQUENCE + 1, EventKind::SubagentStart);
    child.agent_id = Some(CHILD.to_owned());
    reducer.apply(child).unwrap();
    let mut stop = claude_envelope(&expected, FIRST_SEQUENCE + 2, EventKind::SubagentStop);
    stop.agent_id = Some(CHILD.to_owned());
    reducer.apply(stop).unwrap();
    reducer.state.children.get_mut(CHILD).unwrap().turn = TurnState::Running;
    assert_eq!(
        reducer.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );

    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
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
fn compaction_epochs_are_reachable_and_unique() {
    let (expected, mut unknown_prior) = rich_claude_value();
    unknown_prior[fields::LIFECYCLE][fields::COMPACTION][fields::PRIOR_TURN] =
        Value::String(test_values::UNKNOWN.to_owned());
    assert_eq!(
        restore(
            unknown_prior,
            expected,
            HookProviderKind::Claude,
            CLAUDE_VERSION
        )
        .unwrap_err(),
        SnapshotViolation::InvalidState
    );

    let (expected, mut duplicate) = rich_claude_value();
    let root_sequence =
        duplicate[fields::LIFECYCLE][fields::COMPACTION][fields::STARTED_AT].clone();
    duplicate[fields::LIFECYCLE][fields::CHILD_COMPACTIONS][0][fields::PHASE][fields::STARTED_AT] =
        root_sequence;
    assert_eq!(
        restore(duplicate, expected, HookProviderKind::Claude, CLAUDE_VERSION).unwrap_err(),
        SnapshotViolation::InvalidState
    );
}

#[test]
fn codex_specific_impossible_states_are_rejected() {
    let expected = identity();
    let mut initial = Reducer::new(expected, HookProviderKind::Codex, CODEX_VERSION).unwrap();
    initial.state.provider_session_id = Some(test_values::FABRICATED_SESSION.to_owned());
    assert_eq!(
        initial.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );

    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    reducer.state.attention = AttentionState::UserInput;
    assert_eq!(
        reducer.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );
    reducer.state.attention = AttentionState::Permission;
    assert_eq!(
        reducer.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );
    reducer.state.attention = AttentionState::None;
    reducer.state.termination_candidate = Some(TerminationCandidate::Clear);
    reducer.state.turn = TurnState::Unknown;
    assert_eq!(
        reducer.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );
}

#[test]
fn codex_idle_compaction_cannot_gain_attention() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    reducer
        .apply(envelope(
            &expected,
            CODEX_VERSION,
            FIRST_SEQUENCE + 1,
            EventKind::PreCompact,
        ))
        .unwrap();
    reducer.state.attention = AttentionState::Permission;
    assert_eq!(
        reducer.snapshot().unwrap_err(),
        SnapshotViolation::InvalidState
    );
}
