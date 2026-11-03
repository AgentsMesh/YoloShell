use super::{constants::*, fixture::*};
use crate::RuntimeAdmission;
use yolo_hub_agent_protocol::Freshness;
use yolo_hub_connector_protocol::{
    Activation, ConnectorError, EnrollmentStatus, FIRST_PUBLICATION_SEQUENCE,
};
use yolo_hub_identity::{
    AgentTarget, ConnectorGeneration, ConnectorInstanceId, ExecutionId, RuntimeEpoch,
};

#[test]
fn an_enrollment_without_activation_cannot_authorize_a_client_constructed_generation() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrolled = fixture.register(&mut registry);
    let invented = Activation {
        connector_id: enrolled.connector_id,
        generation: ConnectorGeneration::new(),
        agent: AgentTarget {
            address: enrolled.agent,
            execution_id: ExecutionId::new(),
        },
        terminal: enrolled.scope.terminal,
        next_sequence: FIRST_PUBLICATION_SEQUENCE,
    };
    assert_eq!(
        registry.active_scope(invented),
        Err(ConnectorError::StaleGeneration)
    );
    assert!(registry.snapshots().unwrap().is_empty());
    assert!(registry.catalogue().unwrap().primaries.is_empty());
}

#[test]
fn reactivation_after_native_takeover_fails_and_permanently_retires_prior_credential() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrolled = fixture.register(&mut registry);
    let instance = ConnectorInstanceId::new();
    let old = fixture.activate(&mut registry, &enrolled, instance);
    registry
        .publish(
            old,
            publication(old, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    assert_eq!(
        registry.activate(
            enrolled.connector_id,
            enrolled.credential,
            instance,
            RuntimeAdmission::NativeOwned
        ),
        Err(ConnectorError::Conflict)
    );
    assert_eq!(
        registry.list().unwrap()[0].status,
        EnrollmentStatus::Superseded
    );
    assert_eq!(
        registry.snapshots().unwrap()[0].fields().freshness,
        Freshness::Stale
    );
    assert_eq!(
        registry.credential_scope(enrolled.connector_id, enrolled.credential),
        Err(ConnectorError::Unauthorized)
    );
    assert_eq!(
        registry.active_scope(old),
        Err(ConnectorError::StaleGeneration)
    );
}

#[test]
fn rejected_unavailable_or_other_runtime_admission_never_creates_registration() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let mut replacement = fixture.scope.terminal;
    replacement.runtime_epoch = RuntimeEpoch::new();
    assert_eq!(
        registry.register(
            uuid::Uuid::new_v4(),
            fixture.scope.clone(),
            RuntimeAdmission::Available(replacement)
        ),
        Err(ConnectorError::Conflict)
    );
    let enrolled = fixture.register(&mut registry);
    let version = fixture.version(&fixture.member(enrolled.connector_id));
    assert_eq!(
        registry.activate(
            enrolled.connector_id,
            enrolled.credential,
            ConnectorInstanceId::new(),
            RuntimeAdmission::Unavailable
        ),
        Err(ConnectorError::Unavailable)
    );
    assert_eq!(
        fixture.version(&fixture.member(enrolled.connector_id)),
        version
    );
    assert_eq!(
        registry.list().unwrap()[0].status,
        EnrollmentStatus::Enabled
    );
}
