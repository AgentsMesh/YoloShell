use super::{fixture::Fixture, support::published};
use crate::{Registry, state::State};
use yolo_hub_connector_protocol::{ConnectorError, EnrollmentStatus};
use yolo_hub_connector_store::Store;
use yolo_hub_identity::{AgentId, ConnectorId};
use yolo_paths::directory::PrivateDirectory;

#[test]
fn individually_valid_store_members_cannot_claim_one_agent_or_two_enabled_owners_of_a_terminal() {
    for duplicate_agent in [false, true] {
        let fixture = Fixture::new();
        let mut store = Store::<State>::open(
            PrivateDirectory::ensure(&fixture.path).unwrap(),
            fixture.identity,
        )
        .unwrap();
        let (mut first, _) = published();
        first.agent.hub_id = fixture.identity.hub();
        first.scope = fixture.scope.clone();
        store.insert(first.clone(), State::initial()).unwrap();
        let mut second = first.clone();
        second.connector_id = ConnectorId::new();
        second.request_id = uuid::Uuid::new_v4();
        if !duplicate_agent {
            second.agent.agent_id = AgentId::new();
        }
        let mut state = State::initial();
        if duplicate_agent {
            state.status = EnrollmentStatus::Superseded;
        }
        store.insert(second, state).unwrap();
        drop(store);
        assert!(matches!(
            Registry::open(
                PrivateDirectory::open(&fixture.path).unwrap(),
                fixture.identity
            ),
            Err(ConnectorError::Conflict)
        ));
    }
}

#[test]
fn a_retired_registration_and_enabled_successor_may_share_the_terminal_but_not_agent_identity() {
    let fixture = Fixture::new();
    let mut store = Store::<State>::open(
        PrivateDirectory::ensure(&fixture.path).unwrap(),
        fixture.identity,
    )
    .unwrap();
    let (mut first, _) = published();
    first.agent.hub_id = fixture.identity.hub();
    first.scope = fixture.scope.clone();
    let mut state = State::initial();
    state.status = EnrollmentStatus::Superseded;
    store.insert(first.clone(), state).unwrap();
    let mut second = first;
    second.connector_id = ConnectorId::new();
    second.request_id = uuid::Uuid::new_v4();
    second.agent.agent_id = AgentId::new();
    store.insert(second, State::initial()).unwrap();
    drop(store);
    let registry = fixture.open();
    assert_eq!(registry.terminals(), vec![fixture.scope.terminal]);
    assert!(registry.catalogue().unwrap().primaries.is_empty());
}
