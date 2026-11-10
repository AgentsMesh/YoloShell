use super::super::{record, *};
use super::support::Fixture;
use crate::agent::{AgentBinding, AgentRecoveryAvailability, AgentState};

#[test]
fn exited_authority_only_reuses_exact_unknown_or_already_exited_evidence() {
    let fixture = Fixture::new();
    let mut authority = record::begin(None, &fixture.begin).unwrap();
    authority.transcript_path = Some(fixture.transcript(&authority.provider_session_id));
    authority.state = AgentState::Running;
    let mut original = authority.binding(None).unwrap();
    authority.state = AgentState::Exited;
    authority.transcript_path = None;
    original.state = AgentState::Unknown;
    original.recovery_availability = AgentRecoveryAvailability::Unavailable;
    assert!(authority.binding(Some(&original)).is_some());
    assert!(authority.binding(None).is_none());
    let mutations: &[fn(&mut AgentBinding)] = &[
        |value| value.process_pid = None,
        |value| value.process_start_time = None,
        |value| value.provider_session_id = uuid::Uuid::new_v4().to_string(),
        |value| value.state = AgentState::Running,
    ];
    for mutate in mutations {
        let mut changed = original.clone();
        mutate(&mut changed);
        assert!(authority.binding(Some(&changed)).is_none());
    }
    original.state = AgentState::Exited;
    original.process_pid = None;
    original.process_start_time = None;
    let exited = authority.binding(Some(&original)).unwrap();
    assert_eq!(exited.state, AgentState::Exited);
    assert_eq!(exited.process_pid, None);
    assert_eq!(exited.process_start_time, None);
    for pid_field in [true, false] {
        let mut changed = original.clone();
        if pid_field {
            changed.process_pid = Some(authority.process.pid);
        } else {
            changed.process_start_time = Some(authority.process.start_time);
        }
        assert!(authority.binding(Some(&changed)).is_none());
    }
}
