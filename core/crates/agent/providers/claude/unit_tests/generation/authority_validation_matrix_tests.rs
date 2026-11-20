use super::super::{constants::*, record, validation, *};
use super::support::{Fixture, end};
use crate::agent::AgentState;
use uuid::Uuid;

#[path = "event_validation_matrix_tests.rs"]
mod event_validation_matrix_tests;

#[test]
fn generation_begin_requires_every_identity_and_canonical_path_independently() {
    let fixture = Fixture::new();
    let mutations: &[(&str, fn(&mut GenerationBegin))] = &[
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
        (stringify!(helper_capability), |value| {
            value.helper_capability = Uuid::nil()
        }),
        (stringify!(worker_pid), |value| {
            value.worker.pid = u32::default()
        }),
        (stringify!(worker_start), |value| {
            value.worker.start_time = u64::default()
        }),
        (stringify!(agent_pid), |value| {
            value.process.pid = u32::default()
        }),
        (stringify!(agent_start), |value| {
            value.process.start_time = u64::default()
        }),
        (stringify!(handshake_boot_id), |value| {
            value.handshake_boot_id.clear()
        }),
        (stringify!(handshake_deadline_ns), |value| {
            value.handshake_deadline_ns = u64::default()
        }),
        (stringify!(provider_session_id), |value| {
            value.provider_session_id = Uuid::new_v4().simple().to_string()
        }),
        (stringify!(cwd), |value| value.cwd = Default::default()),
        (stringify!(cwd_parent), |value| value.cwd.push("..")),
        (stringify!(data_home), |value| {
            value.data_home = Default::default()
        }),
        (stringify!(data_home_parent), |value| {
            value.data_home.push("..")
        }),
    ];
    validation::validate_begin(&fixture.begin).unwrap();
    for (field, mutate) in mutations {
        let mut changed = fixture.begin.clone();
        mutate(&mut changed);
        assert_eq!(
            validation::validate_begin(&changed)
                .unwrap_err()
                .to_string(),
            AUTHORITY_BEGIN_INVALID,
            "{field}"
        );
    }
}

#[test]
fn persisted_authority_replay_requires_bounded_unique_generation_and_sequence_evidence() {
    let fixture = Fixture::new();
    let mut original = record::begin(None, &fixture.begin).unwrap();
    original.last_event_sequence = crate::claude_hooks::INITIAL_CLAUDE_EVENT_SEQUENCE;
    original.replay_events.push_back(AppliedGenerationEvent {
        sequence: original.last_event_sequence,
        generation: original.generation,
        event: end(&fixture.begin.provider_session_id),
    });
    validation::validate_authority(&original).unwrap();
    let mutations: &[(&str, fn(&mut ClaudeGenerationAuthority))] = &[
        (stringify!(revision), |value| {
            value.revision = INITIAL_REVISION - REVISION_INCREMENT
        }),
        (stringify!(generation), |value| {
            value.generation = INITIAL_GENERATION - REVISION_INCREMENT
        }),
        (stringify!(replay_capacity), |value| {
            value.replay_events =
                std::iter::repeat_n(value.replay_events[0].clone(), MAX_REPLAY_EVENT_IDS + 1)
                    .collect()
        }),
        (stringify!(replay_generation_zero), |value| {
            value.replay_events[0].generation = u64::default()
        }),
        (stringify!(replay_generation_future), |value| {
            value.replay_events[0].generation = value.generation + REVISION_INCREMENT
        }),
        (stringify!(replay_sequence_zero), |value| {
            value.replay_events[0].sequence = u64::default()
        }),
        (stringify!(replay_sequence_future), |value| {
            value.replay_events[0].sequence = value.last_event_sequence + REVISION_INCREMENT
        }),
        (stringify!(replay_event), |value| {
            value.replay_events[0].event = end("invalid")
        }),
        (stringify!(replay_duplicate), |value| {
            value
                .replay_events
                .push_back(value.replay_events[0].clone())
        }),
        (stringify!(binding_revision), |value| {
            value.binding_revision = Some(value.revision + REVISION_INCREMENT)
        }),
        (stringify!(state), |value| {
            value.state = AgentState::WaitingForPermission
        }),
        (stringify!(unknown_transcript), |value| {
            value.transcript_path = Some(value.cwd.clone())
        }),
        (stringify!(unknown_identity), |value| {
            value.transcript_identity = Some(value.settings_identity.object_identity())
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert_eq!(
            validation::validate_authority(&changed)
                .unwrap_err()
                .to_string(),
            AUTHORITY_BEGIN_INVALID,
            "{field}"
        );
    }
}
