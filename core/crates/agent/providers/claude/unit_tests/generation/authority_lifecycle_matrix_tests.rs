use super::super::{constants::AUTHORITY_BEGIN_INVALID, record, validation, *};
use super::support::Fixture;
use crate::agent::AgentState;

#[test]
fn lifecycle_state_requires_the_exact_transcript_and_termination_candidate_shape() {
    let fixture = Fixture::new();
    let initial = record::begin(None, &fixture.begin).unwrap();
    for state in [
        AgentState::Active,
        AgentState::Unknown,
        AgentState::Running,
        AgentState::WaitingForInput,
        AgentState::Exited,
        AgentState::WaitingForPermission,
    ] {
        for path in [false, true] {
            for identity in [false, true] {
                let mut authority = initial.clone();
                authority.state = state;
                authority.transcript_path =
                    path.then(|| fixture.transcript(&authority.provider_session_id));
                authority.transcript_identity =
                    identity.then(|| authority.settings_identity.object_identity());
                let valid = match state {
                    AgentState::Unknown => !path && !identity,
                    AgentState::Running | AgentState::WaitingForInput => path && identity,
                    AgentState::Exited => path == identity,
                    AgentState::Active | AgentState::WaitingForPermission => false,
                };
                assert_eq!(
                    validation::validate_authority(&authority).is_ok(),
                    valid,
                    "{state:?} path={path} identity={identity}"
                );
            }
        }
    }
    for reason in [
        SessionEndReason::Other,
        SessionEndReason::Clear,
        SessionEndReason::Resume,
    ] {
        let mut authority = initial.clone();
        authority.termination_candidate = Some(SessionEndTermination::from_reason(reason));
        validation::validate_authority(&authority).unwrap();
        for state in [AgentState::Running, AgentState::Exited] {
            let mut changed = authority.clone();
            changed.state = state;
            assert_eq!(
                validation::validate_authority(&changed)
                    .unwrap_err()
                    .to_string(),
                AUTHORITY_BEGIN_INVALID
            );
        }
    }
}

#[test]
fn discovery_authority_cannot_claim_a_live_transcript_or_invalid_termination_boundary() {
    let fixture = Fixture::new();
    let mut authority = record::begin(None, &fixture.begin).unwrap();
    authority.discover_session = true;
    validation::validate_authority(&authority).unwrap();
    authority.state = AgentState::Running;
    authority.transcript_path = Some(fixture.transcript(&authority.provider_session_id));
    authority.transcript_identity = Some(authority.settings_identity.object_identity());
    assert!(validation::validate_authority(&authority).is_err());
    authority.discover_session = false;
    validation::validate_authority(&authority).unwrap();
    authority
        .transcript_path
        .as_mut()
        .unwrap()
        .set_extension("invalid");
    assert!(validation::validate_authority(&authority).is_err());
    authority = record::begin(None, &fixture.begin).unwrap();
    authority.termination_candidate = Some(SessionEndTermination {
        reason: SessionEndReason::Other,
        boundary: super::super::termination::SessionEndBoundary::Recovering,
    });
    assert!(validation::validate_authority(&authority).is_err());
}

#[test]
fn authentication_requires_each_capability_and_both_process_identity_components() {
    let fixture = Fixture::new();
    let authority = record::begin(None, &fixture.begin).unwrap();
    let original = authority.authentication();
    let mutations: &[fn(&mut AuthorityAuthentication)] = &[
        |value| value.terminal_id = uuid::Uuid::new_v4(),
        |value| value.runtime_epoch = uuid::Uuid::new_v4(),
        |value| value.invocation_id = uuid::Uuid::new_v4(),
        |value| value.hook_token = uuid::Uuid::new_v4(),
        |value| value.helper_capability = uuid::Uuid::new_v4(),
        |value| value.process.pid = u32::MAX,
        |value| value.process.start_time = u64::MAX,
    ];
    assert!(record::authenticated(&authority, original));
    for mutate in mutations {
        let mut changed = original;
        mutate(&mut changed);
        assert!(!record::authenticated(&authority, changed));
    }
}
