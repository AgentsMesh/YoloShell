use super::test_support::{CODEX_VERSION, envelope, identity};
use super::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

const CWD: &str = "/workspace/project";
const TRANSCRIPT: &str = "/workspace/.codex/transcript.jsonl";
const NON_NORMAL_CWD: &str = "/workspace/../outside";

#[test]
fn snapshot_round_trip_preserves_hook_metadata() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let mut start = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    start.metadata = HookMetadata {
        cwd: Some(PathBuf::from(CWD)),
        transcript_path: Some(PathBuf::from(TRANSCRIPT)),
    };
    reducer.apply(start).unwrap();
    let encoded = serde_json::to_value(reducer.snapshot().unwrap()).unwrap();
    let snapshot = serde_json::from_value::<ReducerSnapshot>(encoded).unwrap();
    let restored =
        Reducer::restore(snapshot, expected, HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let metadata = &restored.state().history[&EventSequence::FIRST].metadata;
    assert_eq!(
        restored.state().current_cwd.as_deref(),
        Some(Path::new(CWD))
    );
    assert_eq!(
        restored.state().current_transcript_path.as_deref(),
        Some(Path::new(TRANSCRIPT))
    );
    assert_eq!(metadata.cwd.as_deref(), Some(Path::new(CWD)));
    assert_eq!(
        metadata.transcript_path.as_deref(),
        Some(Path::new(TRANSCRIPT))
    );
}

#[test]
fn snapshot_preserves_metadata_after_the_origin_event_leaves_the_replay_window() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let mut start = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    start.metadata = HookMetadata {
        cwd: Some(PathBuf::from(CWD)),
        transcript_path: Some(PathBuf::from(TRANSCRIPT)),
    };
    reducer.apply(start).unwrap();
    let last = FIRST_SEQUENCE + MAX_REPLAY_WINDOW as u64;
    for sequence in (FIRST_SEQUENCE + 1)..=last {
        reducer
            .apply(envelope(
                &expected,
                CODEX_VERSION,
                sequence,
                EventKind::UserPromptSubmit,
            ))
            .unwrap();
    }
    assert!(!reducer.state().history.contains_key(&EventSequence::FIRST));
    let restored = Reducer::restore(
        reducer.snapshot().unwrap(),
        expected,
        HookProviderKind::Codex,
        CODEX_VERSION,
    )
    .unwrap();
    assert_eq!(
        restored.state().current_cwd.as_deref(),
        Some(Path::new(CWD))
    );
    assert_eq!(
        restored.state().current_transcript_path.as_deref(),
        Some(Path::new(TRANSCRIPT))
    );
}

#[test]
fn snapshot_rejects_non_normalized_hook_metadata() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let mut start = envelope(
        &expected,
        CODEX_VERSION,
        FIRST_SEQUENCE,
        EventKind::SessionStart {
            source: SessionStartSource::Startup,
        },
    );
    start.metadata.cwd = Some(PathBuf::from(CWD));
    reducer.apply(start).unwrap();
    let mut encoded = serde_json::to_value(reducer.snapshot().unwrap()).unwrap();
    first_history_entry(&mut encoded)[super::constants::snapshot_fields::METADATA]
        [super::constants::snapshot_fields::CWD] = Value::from(NON_NORMAL_CWD);
    let snapshot = serde_json::from_value::<ReducerSnapshot>(encoded).unwrap();
    assert_eq!(
        Reducer::restore(snapshot, expected, HookProviderKind::Codex, CODEX_VERSION).err(),
        Some(SnapshotViolation::InvalidHistory)
    );
}

#[test]
fn snapshot_without_persisted_metadata_is_rejected() {
    let expected = identity();
    let mut reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    reducer
        .apply(envelope(
            &expected,
            CODEX_VERSION,
            FIRST_SEQUENCE,
            EventKind::SessionStart {
                source: SessionStartSource::Startup,
            },
        ))
        .unwrap();
    let mut encoded = serde_json::to_value(reducer.snapshot().unwrap()).unwrap();
    first_history_entry(&mut encoded)
        .as_object_mut()
        .unwrap()
        .remove(super::constants::snapshot_fields::METADATA);
    assert!(serde_json::from_value::<ReducerSnapshot>(encoded).is_err());
}

#[test]
fn current_snapshot_requires_nullable_lifecycle_keys_without_hydration() {
    let expected = identity();
    let reducer = Reducer::new(expected.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let original = serde_json::to_value(reducer.snapshot().unwrap()).unwrap();
    for field in [stringify!(current_cwd), stringify!(current_transcript_path)] {
        assert_eq!(
            original[super::constants::snapshot_fields::LIFECYCLE][field],
            Value::Null
        );
        let mut missing = original.clone();
        missing[super::constants::snapshot_fields::LIFECYCLE]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            serde_json::from_value::<ReducerSnapshot>(missing).is_err(),
            "{field}"
        );
    }
    let snapshot = serde_json::from_value::<ReducerSnapshot>(original).unwrap();
    Reducer::restore(snapshot, expected, HookProviderKind::Codex, CODEX_VERSION).unwrap();
}

fn first_history_entry(value: &mut Value) -> &mut Value {
    value[super::constants::snapshot_fields::HISTORY][super::constants::snapshot_fields::ENTRIES]
        .as_array_mut()
        .unwrap()
        .first_mut()
        .unwrap()
}
