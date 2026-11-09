use super::fixture::Fixture;
use super::*;
use crate::agent::{AgentBinding, AgentKind};

const FIRST_RECORD: usize = 0;
const SCHEMA_STEP: u32 = 1;
const EXTRA_ENTRY: usize = 1;
const NEGATIVE_TIMESTAMP: i64 = -1;
const OTHER_COMPONENT: &str = "other";
const INVALID_SESSION: &str = "not-a-session-id";

#[path = "native_generation_replay_tests.rs"]
mod native_generation_replay_tests;

fn document(fixture: &Fixture) -> NativeGenerations {
    let record = reducer::first(
        &fixture.start,
        fixture.worker,
        &fixture.boot_id,
        &fixture.evidence,
    )
    .unwrap();
    NativeGenerations {
        schema_version: SCHEMA_VERSION,
        active_invocation: record.invocation_id,
        invocations: vec![record],
    }
}

#[test]
fn native_document_requires_supported_schema_a_bounded_inventory_and_an_active_member() {
    let fixture = Fixture::new();
    let original = document(&fixture);
    original.validate().unwrap();
    let mutations: &[fn(&mut NativeGenerations)] = &[
        |value| value.schema_version += SCHEMA_STEP,
        |value| value.active_invocation = Uuid::nil(),
        |value| value.active_invocation = Uuid::new_v4(),
        |value| value.invocations.clear(),
        |value| {
            value.invocations = vec![
                value.invocations[FIRST_RECORD].clone();
                super::super::constants::MAX_INVOCATION_AUTHORITIES
                    + EXTRA_ENTRY
            ]
        },
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(changed.validate().unwrap_err().to_string(), INVALID);
    }
}

#[test]
fn each_native_record_requires_complete_scope_worker_and_replay_identity() {
    let fixture = Fixture::new();
    let original = document(&fixture);
    let mutations: &[(&str, fn(&mut ClaudeNativeGeneration))] = &[
        ("schema", |value| value.schema_version += SCHEMA_STEP),
        ("session", |value| value.scope.terminal_id = Uuid::nil()),
        ("runtime", |value| value.scope.runtime_epoch = Uuid::nil()),
        ("agent_pid", |value| value.scope.agent.pid = u32::default()),
        ("agent.start", |value| {
            value.scope.agent.start_time = u64::default()
        }),
        (stringify!(worker.pid), |value| {
            value.worker.pid = u32::default()
        }),
        ("worker_start", |value| {
            value.worker.start_time = u64::default()
        }),
        ("boot", |value| value.boot_id.clear()),
        ("revision", |value| value.revision = u64::default()),
        ("sequence", |value| {
            value.last_event_sequence = u64::default()
        }),
        ("missing_replay", |value| value.replay_events.clear()),
        ("oversized_replay", |value| {
            value.replay_events = std::iter::repeat_n(
                value.replay_events[FIRST_RECORD].clone(),
                super::super::constants::MAX_REPLAY_EVENT_IDS + EXTRA_ENTRY,
            )
            .collect()
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed.invocations[FIRST_RECORD]);
        assert_eq!(
            changed.validate().unwrap_err().to_string(),
            INVALID,
            "{field}"
        );
    }
    original.validate().unwrap();
}

#[test]
fn every_native_binding_field_is_bound_to_the_record_and_transcript_contract() {
    let fixture = Fixture::new();
    let original = document(&fixture);
    let mutations: &[(&str, fn(&mut AgentBinding))] = &[
        ("session", |value| value.terminal_id = Uuid::new_v4()),
        ("kind", |value| value.kind = AgentKind::Codex),
        ("executable", |value| {
            value.executable = OTHER_COMPONENT.into()
        }),
        ("schema", |value| value.schema_version += SCHEMA_STEP),
        ("activity", |value| value.state = AgentState::Unknown),
        ("detected_at", |value| {
            value.detected_at_ms = NEGATIVE_TIMESTAMP
        }),
        ("updated_at", |value| {
            value.updated_at_ms = value.detected_at_ms + NEGATIVE_TIMESTAMP
        }),
        (crate::constants::PROCESS_PID_FIELD, |value| {
            value.process_pid = None
        }),
        ("process_start", |value| value.process_start_time = None),
        ("provider_session", |value| {
            value.provider_session_id = INVALID_SESSION.into()
        }),
        ("transcript", |value| {
            value.evidence_path = OTHER_COMPONENT.into()
        }),
        (crate::constants::DATA_HOME_FIELD, |value| {
            value.data_home.push_str(OTHER_COMPONENT)
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed.invocations[FIRST_RECORD].binding);
        assert_eq!(
            changed.validate().unwrap_err().to_string(),
            INVALID,
            "{field}"
        );
    }
    let mut boundary = original;
    boundary.invocations[FIRST_RECORD].binding.detected_at_ms = i64::default();
    boundary.invocations[FIRST_RECORD].binding.updated_at_ms = i64::default();
    boundary.validate().unwrap();
}

#[test]
fn previous_replay_schema_migrates_before_validation() {
    let fixture = Fixture::new();
    let mut value = document(&fixture);
    for request in &mut value.invocations[FIRST_RECORD].replay_events {
        request.schema_version = crate::claude_hooks::PREVIOUS_CLAUDE_NATIVE_SCHEMA_VERSION;
    }
    assert!(value.validate().is_err());
    value.migrate_replay_schema();
    value.validate().unwrap();
    assert!(value.invocations[FIRST_RECORD]
        .replay_events
        .iter()
        .all(|request| request.schema_version == crate::claude_hooks::CLAUDE_NATIVE_SCHEMA_VERSION));
}
