use super::super::{binding, record, *};
use super::support::Fixture;
use crate::agent::{AgentBinding, AgentKind, AgentRecoveryAvailability, AgentState};
use uuid::Uuid;

const OTHER: &str = "other";
const STEP: u64 = 1;

#[test]
fn binding_subject_and_unknown_transition_reject_every_foreign_identity_field() {
    let fixture = Fixture::new();
    let authority = record::begin(None, &fixture.begin).unwrap();
    let mut with_transcript = authority.clone();
    with_transcript.transcript_path = Some(fixture.transcript(&authority.provider_session_id));
    with_transcript.state = AgentState::Running;
    let mut original = with_transcript.binding(None).unwrap();
    original.state = AgentState::Unknown;
    original.recovery_availability = AgentRecoveryAvailability::Unavailable;
    assert!(authority.matches_unknown_binding(&original));
    let mutations: &[(&str, fn(&mut AgentBinding))] = &[
        (stringify!(schema_version), |value| {
            value.schema_version += 1
        }),
        (stringify!(kind), |value| value.kind = AgentKind::Codex),
        (stringify!(terminal_id), |value| {
            value.terminal_id = Uuid::new_v4()
        }),
        (stringify!(provider_session_id), |value| {
            value.provider_session_id = Uuid::new_v4().to_string()
        }),
        (stringify!(executable), |value| {
            value.executable = OTHER.into()
        }),
        (stringify!(cwd), |value| value.cwd.push_str(OTHER)),
        (stringify!(data_home), |value| {
            value.data_home.push_str(OTHER)
        }),
        (stringify!(evidence_path), |value| {
            value.evidence_path = OTHER.into()
        }),
        (stringify!(process_pid), |value| value.process_pid = None),
        (stringify!(process_start_time), |value| {
            value.process_start_time = None
        }),
        (stringify!(timestamps), |value| {
            value.updated_at_ms = value.detected_at_ms - 1
        }),
    ];
    for (field, mutate) in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(!authority.matches_unknown_binding(&changed), "{field}");
        assert!(
            authority.transition_binding(Some(&changed)).is_none(),
            "{field}"
        );
    }
    assert!(authority.transition_binding(None).is_none());
    for state in [AgentState::Running, AgentState::Exited] {
        let mut changed = original.clone();
        changed.state = state;
        assert!(!authority.matches_unknown_binding(&changed));
    }
    for availability in [
        AgentRecoveryAvailability::Available,
        AgentRecoveryAvailability::UnmanagedLaunch,
    ] {
        let mut changed = original.clone();
        changed.recovery_availability = availability;
        assert!(!authority.matches_unknown_binding(&changed));
    }
}

#[test]
fn published_binding_payload_requires_exact_subject_status_and_live_process_identity() {
    let fixture = Fixture::new();
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let event = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
    fixture
        .store
        .apply_in_namespace(&fixture.lease, fixture.authentication(), &event)
        .unwrap();
    let mut authority = fixture
        .store
        .load_in_namespace(&fixture.lease)
        .unwrap()
        .unwrap();
    authority.binding_revision = Some(authority.revision);
    let original = authority.binding(None).unwrap();
    assert!(authority.matches_binding(&original));
    let mutations: &[fn(&mut AgentBinding)] = &[
        |value| value.schema_version += 1,
        |value| value.kind = AgentKind::Codex,
        |value| value.terminal_id = Uuid::new_v4(),
        |value| value.provider_session_id = Uuid::new_v4().to_string(),
        |value| value.executable = OTHER.into(),
        |value| value.cwd.push_str(OTHER),
        |value| value.data_home.push_str(OTHER),
        |value| value.state = AgentState::Unknown,
        |value| value.recovery_availability = AgentRecoveryAvailability::Unavailable,
        |value| value.evidence_path.push_str(OTHER),
        |value| value.process_pid = None,
        |value| value.process_start_time = value.process_start_time.map(|start| start + STEP),
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(!authority.matches_binding_payload(&changed));
    }
    authority.discover_session = true;
    assert!(!authority.matches_binding_payload(&original));
    authority.discover_session = false;
    authority.binding_revision = None;
    assert!(!authority.matches_binding(&original));
    assert!(binding::subject_matches(&authority, &original));
}
