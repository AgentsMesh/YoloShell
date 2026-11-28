use crate::{constants::*, fixture::*};
use yolo_hub_agent_protocol::Freshness;
use yolo_hub_connector_protocol::{
    ConnectorCredential, ConnectorError, FIRST_PUBLICATION_SEQUENCE,
};
use yolo_hub_identity::{
    ConnectorGeneration, ConnectorId, ConnectorInstanceId, ExecutionId, RuntimeEpoch,
};

#[test]
fn wrong_credential_never_reconciles_or_changes_existing_enrollment() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let path = fixture.member(enrollment.connector_id);
    let before = fixture.version(&path);
    let wrong = ConnectorCredential::generate();
    assert_eq!(
        registry.credential_scope(enrollment.connector_id, wrong),
        Err(ConnectorError::Unauthorized)
    );
    assert_eq!(
        registry.activate(
            enrollment.connector_id,
            wrong,
            ConnectorInstanceId::new(),
            yolo_hub_connector_registry::RuntimeAdmission::NativeOwned
        ),
        Err(ConnectorError::Unauthorized)
    );
    assert_eq!(
        registry
            .credential_scope(enrollment.connector_id, enrollment.credential)
            .unwrap(),
        fixture.scope
    );
    assert_eq!(fixture.version(&path), before);
}

#[test]
fn same_instance_reactivation_fences_old_connection_and_preserves_agent_execution() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let instance = ConnectorInstanceId::new();
    let old = fixture.activate(&mut registry, &enrollment, instance);
    let previous = registry
        .publish(
            old,
            publication(old, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let current = fixture.activate(&mut registry, &enrollment, instance);
    assert_eq!(current.agent, old.agent);
    assert_ne!(current.generation, old.generation);
    assert_eq!(
        registry.snapshots().unwrap()[0].fields().freshness,
        Freshness::Stale
    );
    assert_eq!(
        registry.active_scope(old),
        Err(ConnectorError::StaleGeneration)
    );
    assert_eq!(
        registry.publish(
            old,
            publication(old, FIRST_PUBLICATION_SEQUENCE + STEP),
            fixture.admission(),
            LATER
        ),
        Err(ConnectorError::StaleGeneration)
    );
    let ack = registry
        .publish(
            current,
            publication(current, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            LATER,
        )
        .unwrap();
    assert_ne!(ack.cursor.stream(), previous.cursor.stream());
    let path = fixture.member(enrollment.connector_id);
    let before = fixture.version(&path);
    registry.disconnect(old).unwrap();
    assert_eq!(fixture.version(&path), before);
    assert!(registry.active_scope(current).is_ok());
    assert_eq!(
        registry.snapshots().unwrap()[0].fields().freshness,
        Freshness::Current
    );
}

#[test]
fn new_instance_gets_new_execution_but_keeps_logical_agent_and_waits_for_publication() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let old = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    registry
        .publish(
            old,
            publication(old, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let new = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    assert_eq!(new.agent.address, old.agent.address);
    assert_ne!(new.agent.execution_id, old.agent.execution_id);
    let stale = registry.snapshots().unwrap();
    assert_eq!(stale[0].cursor().target(), old.agent);
    assert_eq!(stale[0].fields().freshness, Freshness::Stale);
    registry
        .publish(
            new,
            publication(new, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            LATER,
        )
        .unwrap();
    assert_eq!(
        registry.snapshots().unwrap()[0].cursor().target(),
        new.agent
    );
}

#[test]
fn exited_instance_cannot_resurrect_but_explicit_new_instance_can_publish() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let instance = ConnectorInstanceId::new();
    let active = fixture.activate(&mut registry, &enrollment, instance);
    registry
        .publish(
            active,
            exited(publication(active, FIRST_PUBLICATION_SEQUENCE)),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    assert!(registry.catalogue().unwrap().enabled_targets.is_empty());
    assert_eq!(
        registry.publish(
            active,
            publication(active, FIRST_PUBLICATION_SEQUENCE + STEP),
            fixture.admission(),
            LATER
        ),
        Err(ConnectorError::Conflict)
    );
    let again = fixture.activate(&mut registry, &enrollment, instance);
    assert_eq!(
        registry.publish(
            again,
            publication(again, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            LATER
        ),
        Err(ConnectorError::Conflict)
    );
    let new = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    assert_ne!(new.agent.execution_id, active.agent.execution_id);
    registry
        .publish(
            new,
            publication(new, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            LATER,
        )
        .unwrap();
}

#[test]
fn live_scope_validates_complete_activation_and_not_only_the_connector_id() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let active = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    let mutations: &[fn(&mut yolo_hub_connector_protocol::Activation)] = &[
        |a| a.connector_id = ConnectorId::new(),
        |a| a.generation = ConnectorGeneration::new(),
        |a| a.agent.execution_id = ExecutionId::new(),
        |a| a.terminal.runtime_epoch = RuntimeEpoch::new(),
        |a| a.agent.address.agent_id = yolo_hub_identity::AgentId::new(),
    ];
    for mutate in mutations {
        let mut changed = active;
        mutate(&mut changed);
        assert!(registry.active_scope(changed).is_err());
    }
    assert_eq!(registry.active_scope(active).unwrap(), fixture.scope);
}
