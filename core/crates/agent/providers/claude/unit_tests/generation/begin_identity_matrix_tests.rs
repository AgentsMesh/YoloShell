use super::super::{
    ClaudeGenerationAuthority, InvocationMode, SessionEndReason, SessionEndTermination, record,
};
use super::support::Fixture;
use crate::agent::AgentState;
use uuid::Uuid;

const IDENTITY_STEP: u64 = 1;
const PID_STEP: u32 = 1;
const OTHER_COMPONENT: &str = "other";

#[test]
fn matching_begin_requires_every_invocation_process_and_path_identity() {
    let fixture = Fixture::new();
    let original = record::begin(None, &fixture.begin).unwrap();
    assert!(record::matches_begin(&original, &fixture.begin));
    let mutations: &[(&str, fn(&mut ClaudeGenerationAuthority))] = &[
        (stringify!(terminal_id), |value| {
            value.terminal_id = Uuid::new_v4()
        }),
        (stringify!(runtime_epoch), |value| {
            value.runtime_epoch = Uuid::new_v4()
        }),
        (stringify!(invocation_id), |value| {
            value.invocation_id = Uuid::new_v4()
        }),
        ("hook_token", |value| value.hook_token = Uuid::new_v4()),
        ("helper_capability", |value| {
            value.helper_capability = Uuid::new_v4()
        }),
        (stringify!(worker.pid), |value| value.worker.pid += PID_STEP),
        ("worker_start", |value| {
            value.worker.start_time += IDENTITY_STEP
        }),
        (stringify!(process.pid), |value| {
            value.process.pid += PID_STEP
        }),
        ("process_start", |value| {
            value.process.start_time += IDENTITY_STEP
        }),
        (stringify!(provider_session_id), |value| {
            value.provider_session_id = Uuid::new_v4().to_string()
        }),
        ("discover_session", |value| value.discover_session = true),
        ("invocation_mode", |value| {
            value.mode = InvocationMode::Resume
        }),
        ("cwd", |value| value.cwd.push(OTHER_COMPONENT)),
        (crate::constants::DATA_HOME_FIELD, |value| {
            value.data_home.push(OTHER_COMPONENT)
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(!record::matches_begin(&changed, &fixture.begin), "{field}");
    }
    let other = Fixture::new();
    let mut changed = original;
    changed.settings_identity = other.begin.settings_identity;
    assert!(!record::matches_begin(&changed, &fixture.begin));
}

#[test]
fn an_already_published_or_terminated_invocation_cannot_replay_begin() {
    let fixture = Fixture::new();
    let original = record::begin(None, &fixture.begin).unwrap();
    let mutations: &[(&str, fn(&mut ClaudeGenerationAuthority))] = &[
        (stringify!(transcript_path), |value| {
            value.transcript_path = Some(value.cwd.join(OTHER_COMPONENT))
        }),
        (stringify!(transcript_identity), |value| {
            value.transcript_identity = Some(value.settings_identity.object_identity())
        }),
        ("binding_revision", |value| {
            value.binding_revision = Some(value.revision)
        }),
        ("event_sequence", |value| {
            value.last_event_sequence += IDENTITY_STEP
        }),
        ("state", |value| value.state = AgentState::Running),
        ("termination", |value| {
            value.termination_candidate =
                Some(SessionEndTermination::from_reason(SessionEndReason::Other))
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(!record::matches_begin(&changed, &fixture.begin), "{field}");
    }
    assert!(record::matches_begin(&original, &fixture.begin));
}
