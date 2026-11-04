use crate::{constants::*, fixture::*};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::Freshness;
use yolo_hub_connector_protocol::FIRST_PUBLICATION_SEQUENCE;
use yolo_hub_connector_registry::RuntimeAdmission;
use yolo_hub_identity::{ConnectorInstanceId, TerminalId};

#[test]
fn single_session_refresh_does_not_deactivate_unobserved_connectors() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let first = fixture.register(&mut registry);
    let first = fixture.activate(&mut registry, &first, ConnectorInstanceId::new());
    let mut scope = fixture.scope.clone();
    scope.terminal.address.terminal_id = TerminalId::new();
    let admission = RuntimeAdmission::Available(scope.terminal);
    let second = registry
        .register(uuid::Uuid::new_v4(), scope, admission)
        .unwrap();
    let second = registry
        .activate(
            second.connector_id,
            second.credential,
            ConnectorInstanceId::new(),
            admission,
        )
        .unwrap();
    registry
        .publish(
            first,
            publication(first, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    registry
        .publish(
            second,
            publication(second, FIRST_PUBLICATION_SEQUENCE),
            admission,
            NOW,
        )
        .unwrap();
    let before = fixture.version(&fixture.member(second.connector_id));
    registry
        .reconcile_observed(&BTreeMap::from([(
            first.terminal,
            RuntimeAdmission::Unavailable,
        )]))
        .unwrap();
    assert!(registry.active_scope(first).is_err());
    assert!(registry.active_scope(second).is_ok());
    assert_eq!(
        fixture.version(&fixture.member(second.connector_id)),
        before
    );
    let catalogue = registry.catalogue().unwrap();
    assert_eq!(
        catalogue
            .agents
            .iter()
            .find(|agent| agent.cursor().target() == second.agent)
            .unwrap()
            .fields()
            .freshness,
        Freshness::Current
    );
    registry.reconcile_observed(&BTreeMap::new()).unwrap();
    assert!(registry.active_scope(second).is_ok());
}
