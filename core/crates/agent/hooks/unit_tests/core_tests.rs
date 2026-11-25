use std::path::PathBuf;
use uuid::Uuid;
use yolo_hooks::{
    ClaudeEventMapper, CodexEventMapper, EventKind, EventMapper, EventSequence, HookEnvelope,
    HookIdentity, HookMetadata, HookViolation, ProcessIdentity, ProviderEventMapper, HookProviderKind,
    ProviderVersion,
};

const SESSION: Uuid = Uuid::from_u128(1);
const EPOCH: Uuid = Uuid::from_u128(2);
const INVOCATION: Uuid = Uuid::from_u128(3);
const TOKEN: Uuid = Uuid::from_u128(4);
const PROCESS_ID: u32 = 11;
const PROCESS_START: u64 = 12;

fn identity() -> HookIdentity {
    HookIdentity {
        terminal_id: SESSION,
        runtime_epoch: EPOCH,
        invocation_id: INVOCATION,
        hook_token: TOKEN,
        process: ProcessIdentity::new(PROCESS_ID, PROCESS_START),
        generation: yolo_hooks::Generation::INITIAL,
    }
}

#[test]
fn identity_and_metadata_fail_closed() {
    assert!(identity().validate().is_ok());
    let invalid = HookIdentity {
        terminal_id: Uuid::nil(),
        ..identity()
    };
    assert_eq!(invalid.validate(), Err(HookViolation::InvalidIdentity));
    let valid = HookMetadata {
        cwd: Some(PathBuf::from("/workspace")),
        transcript_path: Some(PathBuf::from("/workspace/transcript.jsonl")),
    };
    assert!(valid.is_valid());
    assert!(
        !HookMetadata {
            cwd: Some(PathBuf::from("relative")),
            transcript_path: None,
        }
        .is_valid()
    );
}

#[test]
fn every_hook_identity_component_is_required_independently() {
    let original = identity();
    let mutations: &[(&str, fn(&mut HookIdentity))] = &[
        (stringify!(terminal_id), |value| {
            value.terminal_id = Uuid::nil()
        }),
        (stringify!(runtime_epoch), |value| {
            value.runtime_epoch = Uuid::nil()
        }),
        (stringify!(invocation_id), |value| {
            value.invocation_id = Uuid::nil()
        }),
        (stringify!(hook_token), |value| {
            value.hook_token = Uuid::nil()
        }),
        (stringify!(pid), |value| value.process.pid = u32::default()),
        (stringify!(start_time), |value| {
            value.process.start_time = u64::default()
        }),
        (stringify!(generation), |value| {
            value.generation = yolo_hooks::Generation(u64::default())
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(
            changed.validate(),
            Err(HookViolation::InvalidIdentity),
            "{field}"
        );
    }
    original.validate().unwrap();
}

#[test]
fn provider_versions_compare_each_numeric_component_and_sequences_never_wrap() {
    const VERSION: ProviderVersion = ProviderVersion::new(2, 3, 4);
    for (candidate, accepted) in [
        (ProviderVersion::new(1, u16::MAX, u16::MAX), false),
        (ProviderVersion::new(2, 2, u16::MAX), false),
        (ProviderVersion::new(2, 3, 3), false),
        (VERSION, true),
        (ProviderVersion::new(2, 3, 5), true),
        (ProviderVersion::new(2, 4, 0), true),
        (ProviderVersion::new(3, 0, 0), true),
    ] {
        assert_eq!(candidate.at_least(VERSION), accepted, "{candidate:?}");
    }
    assert_eq!(EventSequence(u64::MAX).next(), None);
    for (pid, start, valid) in [
        (0, PROCESS_START, false),
        (PROCESS_ID, 0, false),
        (PROCESS_ID, PROCESS_START, true),
    ] {
        assert_eq!(ProcessIdentity::new(pid, start).is_valid(), valid);
    }
}

#[test]
fn provider_event_mappers_keep_provider_specific_events() {
    let codex = CodexEventMapper;
    let claude = ClaudeEventMapper;
    assert_eq!(codex.provider(), HookProviderKind::Codex);
    assert_eq!(claude.provider(), HookProviderKind::Claude);
    assert!(matches!(
        codex.map("SessionStart"),
        Some(EventKind::SessionStart { .. })
    ));
    assert!(matches!(
        claude.map("PermissionDenied"),
        Some(EventKind::PermissionDenied)
    ));
    assert!(codex.map("PermissionDenied").is_none());
}

#[test]
fn envelopes_use_monotonic_sequence_contract() {
    let sequence = EventSequence::FIRST;
    assert_eq!(sequence.next(), Some(EventSequence(sequence.0 + 1)));
    let envelope = HookEnvelope {
        schema_version: yolo_hooks::CURRENT_SCHEMA_VERSION,
        event_id: Uuid::from_u128(5),
        sequence,
        provider: HookProviderKind::Codex,
        provider_version: ProviderVersion::new(1, 0, 0),
        identity: identity(),
        provider_session_id: "session".into(),
        turn_id: None,
        agent_id: None,
        tool_name: None,
        tool_use_id: None,
        stop_hook_active: None,
        metadata: HookMetadata::default(),
        event: EventKind::UserPromptSubmit,
    };
    assert!(envelope.has_bounded_storage_encoding());
}

#[test]
fn normalized_envelope_requires_metadata_even_when_paths_are_absent() {
    let envelope = HookEnvelope {
        schema_version: yolo_hooks::CURRENT_SCHEMA_VERSION,
        event_id: Uuid::from_u128(6),
        sequence: EventSequence::FIRST,
        provider: HookProviderKind::Codex,
        provider_version: ProviderVersion::new(1, 0, 0),
        identity: identity(),
        provider_session_id: "session".into(),
        turn_id: None,
        agent_id: None,
        tool_name: None,
        tool_use_id: None,
        stop_hook_active: None,
        metadata: HookMetadata::default(),
        event: EventKind::UserPromptSubmit,
    };
    let mut value = serde_json::to_value(&envelope).unwrap();
    assert_eq!(
        serde_json::from_value::<HookEnvelope>(value.clone()).unwrap(),
        envelope
    );
    value.as_object_mut().unwrap().remove(stringify!(metadata));
    assert!(serde_json::from_value::<HookEnvelope>(value).is_err());
}
