use super::constants::{REDUCER_SNAPSHOT_SCHEMA_VERSION, snapshot_fields as fields};
use super::test_support::{CHILD, CLAUDE_VERSION, CODEX_VERSION, envelope, identity, started};
use super::*;
use serde_json::{Value, from_value, to_value};

fn round_trip(
    reducer: &Reducer,
    expected: HookIdentity,
    provider: HookProviderKind,
    version: ProviderVersion,
) -> Reducer {
    let encoded = to_value(reducer.snapshot().unwrap()).unwrap();
    let snapshot = from_value::<ReducerSnapshot>(encoded).unwrap();
    Reducer::restore(snapshot, expected, provider, version).unwrap()
}

#[test]
fn snapshot_round_trip_preserves_complex_authority() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    let start = super::test_support::claude_envelope(
        &expected,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    reducer.apply(start).unwrap();
    let mut child = super::test_support::claude_envelope(
        &expected,
        FIRST_SEQUENCE + 1,
        EventKind::SubagentStart,
    );
    child.agent_id = Some(CHILD.to_owned());
    reducer.apply(child).unwrap();
    let mut compact =
        super::test_support::claude_envelope(&expected, FIRST_SEQUENCE + 2, EventKind::PreCompact);
    compact.agent_id = Some(CHILD.to_owned());
    reducer.apply(compact).unwrap();

    let restored = round_trip(&reducer, expected, HookProviderKind::Claude, CLAUDE_VERSION);
    assert_eq!(restored.state(), reducer.state());
    assert_eq!(restored.revision(), reducer.revision());
}

#[test]
fn replay_does_not_advance_revision() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let start = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    let initial = reducer.revision();
    reducer.apply(start.clone()).unwrap();
    assert_eq!(reducer.revision(), initial + FIRST_SEQUENCE);
    let applied = reducer.revision();
    assert_eq!(reducer.apply(start), Ok(ApplyDisposition::Replay));
    assert_eq!(reducer.revision(), applied);
}

#[test]
fn exited_and_revoked_authorities_restore_canonically() {
    let expected = identity();
    let mut exited = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut exited, &expected);
    exited.process_exit();
    let restored = round_trip(
        &exited,
        expected.clone(),
        HookProviderKind::Codex,
        CODEX_VERSION,
    );
    assert_eq!(restored.state(), exited.state());

    let mut revoked = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut revoked, &expected);
    let gap = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_SEQUENCE + 2,
        EventKind::UserPromptSubmit,
    );
    assert_eq!(revoked.apply(gap), Err(HookViolation::SequenceGap));
    let restored = round_trip(&revoked, expected, HookProviderKind::Codex, CODEX_VERSION);
    assert_eq!(restored.state(), revoked.state());
}

#[test]
fn revocation_after_exit_preserves_its_distinct_revision() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &expected);
    assert_eq!(reducer.process_exit(), ProcessExitDisposition::Unconfirmed);
    let wrong_process = ProcessIdentity::new(
        expected.process.pid,
        expected.process.start_time + FIRST_SEQUENCE,
    );
    assert_eq!(
        reducer.process_exit_with_identity(wrong_process),
        Err(HookViolation::ProcessMismatch)
    );
    let restored = round_trip(&reducer, expected, HookProviderKind::Codex, CODEX_VERSION);
    assert_eq!(restored.state(), reducer.state());
    assert_eq!(restored.revision(), reducer.revision());
}

#[test]
fn claude_idle_attention_and_idle_compaction_remain_reachable() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Claude, CLAUDE_VERSION).unwrap();
    reducer
        .apply(super::test_support::claude_envelope(
            &expected,
            FIRST_SEQUENCE,
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        ))
        .unwrap();
    reducer
        .apply(super::test_support::claude_envelope(
            &expected,
            FIRST_SEQUENCE + 1,
            EventKind::Notification {
                kind: NotificationKind::PermissionPrompt,
            },
        ))
        .unwrap();
    assert!(reducer.snapshot().is_ok());
    reducer
        .apply(super::test_support::claude_envelope(
            &expected,
            FIRST_SEQUENCE + 2,
            EventKind::PreCompact,
        ))
        .unwrap();
    let restored = round_trip(&reducer, expected, HookProviderKind::Claude, CLAUDE_VERSION);
    assert_eq!(restored.state(), reducer.state());
}

#[test]
fn restore_binds_identity_provider_version_and_schema() {
    let expected = identity();
    let reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let value = to_value(reducer.snapshot().unwrap()).unwrap();
    assert_restore_error(
        value.clone(),
        identity(),
        HookProviderKind::Codex,
        CODEX_VERSION,
        SnapshotViolation::IdentityMismatch,
    );
    assert_restore_error(
        value.clone(),
        expected.clone(),
        HookProviderKind::Claude,
        CODEX_VERSION,
        SnapshotViolation::ProviderMismatch,
    );
    assert_restore_error(
        value.clone(),
        expected.clone(),
        HookProviderKind::Codex,
        CLAUDE_VERSION,
        SnapshotViolation::VersionMismatch,
    );
    let mut wrong_schema = value;
    wrong_schema[fields::SCHEMA_VERSION] =
        Value::from(u64::from(REDUCER_SNAPSHOT_SCHEMA_VERSION + 1));
    assert_restore_error(
        wrong_schema,
        expected,
        HookProviderKind::Codex,
        CODEX_VERSION,
        SnapshotViolation::SchemaMismatch,
    );
}

fn assert_restore_error(
    value: Value,
    expected: HookIdentity,
    provider: HookProviderKind,
    version: ProviderVersion,
    error: SnapshotViolation,
) {
    let snapshot = from_value::<ReducerSnapshot>(value).unwrap();
    assert_eq!(
        Reducer::restore(snapshot, expected, provider, version).err(),
        Some(error)
    );
}
