use crate::{constants::*, fixture::*};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AgentCapabilities, Freshness, ObservationSource, Reachability, RecoveryStatus,
};
use yolo_hub_connector_protocol::{ConnectorError, EnrollmentStatus, FIRST_PUBLICATION_SEQUENCE};
use yolo_hub_connector_registry::RuntimeAdmission;
use yolo_hub_identity::ConnectorInstanceId;

#[test]
fn registration_and_activation_are_not_agent_observations() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    assert!(registry.snapshots().unwrap().is_empty());
    assert!(registry.catalogue().unwrap().primaries.is_empty());
    assert!(registry.last_known().is_empty());
    assert_eq!(registry.terminals(), vec![fixture.scope.terminal]);
    let listing = registry.list().unwrap();
    assert_eq!(listing.len(), 1);
    assert_eq!(listing[0].status, EnrollmentStatus::Enabled);
    assert_eq!(listing[0].agent, enrollment.agent);
    let activation = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    assert!(registry.snapshots().unwrap().is_empty());
    registry
        .publish(
            activation,
            publication(activation, FIRST_PUBLICATION_SEQUENCE),
            fixture.admission(),
            NOW,
        )
        .unwrap();
    let catalogue = registry.catalogue().unwrap();
    let snapshot = &catalogue.agents[0];
    assert_eq!(snapshot.cursor().target(), activation.agent);
    assert_eq!(snapshot.fields().terminal, activation.terminal);
    assert_eq!(snapshot.fields().source, ObservationSource::Connector);
    assert_eq!(
        snapshot.fields().capabilities,
        AgentCapabilities::UNSUPPORTED
    );
    assert_eq!(snapshot.fields().recovery, RecoveryStatus::Unsupported);
    assert!(snapshot.fields().interaction.is_none());
    assert_eq!(snapshot.fields().freshness, Freshness::Current);
    assert_eq!(snapshot.fields().reachability, Reachability::Reachable);
    assert_eq!(
        catalogue.primaries.get(&activation.terminal),
        Some(&activation.agent)
    );
    assert_eq!(
        catalogue.enabled_targets,
        vec![(activation.terminal.address.terminal_id, activation.agent)]
    );
}

#[test]
fn exact_registration_retry_returns_same_ids_and_secret_without_rewriting_files() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let request = Uuid::new_v4();
    let first = registry
        .register(request, fixture.scope.clone(), fixture.admission())
        .unwrap();
    let path = fixture.member(first.connector_id);
    let before = fixture.version(&path);
    assert_eq!(
        registry
            .register(request, fixture.scope.clone(), fixture.admission())
            .unwrap(),
        first
    );
    assert_eq!(fixture.version(&path), before);
    let mut changed = fixture.scope.clone();
    changed.provider =
        yolo_hub_agent_protocol::ObservedProvider::new(OTHER_PROVIDER.into(), None, None)
            .unwrap();
    assert_eq!(
        registry.register(request, changed, fixture.admission()),
        Err(ConnectorError::Conflict)
    );
    assert_eq!(
        registry.register(Uuid::new_v4(), fixture.scope.clone(), fixture.admission()),
        Err(ConnectorError::Conflict)
    );
    assert_eq!(registry.list().unwrap().len(), 1);
}

#[test]
fn invalid_registration_or_native_runtime_never_creates_an_enrollment() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    assert_eq!(
        registry.register(Uuid::nil(), fixture.scope.clone(), fixture.admission()),
        Err(ConnectorError::InvalidRequest)
    );
    for admission in [
        RuntimeAdmission::NativeOwned,
        RuntimeAdmission::Retired,
        RuntimeAdmission::Unavailable,
    ] {
        assert!(
            registry
                .register(Uuid::new_v4(), fixture.scope.clone(), admission)
                .is_err()
        );
    }
    let mut other = fixture.scope.clone();
    other.terminal.address.hub_id = yolo_hub_identity::HubId::new();
    assert!(
        registry
            .register(
                Uuid::new_v4(),
                other.clone(),
                RuntimeAdmission::Available(other.terminal)
            )
            .is_err()
    );
    assert!(registry.list().unwrap().is_empty());
}

#[test]
fn revoke_is_exact_idempotent_cas_and_never_restores_primary_authority() {
    let fixture = Fixture::new();
    let mut registry = fixture.open();
    let enrollment = fixture.register(&mut registry);
    let activation = fixture.activate(&mut registry, &enrollment, ConnectorInstanceId::new());
    let published = publication(activation, FIRST_PUBLICATION_SEQUENCE);
    registry
        .publish(activation, published.clone(), fixture.admission(), NOW)
        .unwrap();
    let request = Uuid::new_v4();
    let revoked = registry
        .revoke(request, enrollment.connector_id, enrollment.revision)
        .unwrap();
    assert_eq!(revoked.revision, enrollment.revision + STEP);
    assert_eq!(revoked.status, EnrollmentStatus::Revoked);
    let path = fixture.member(enrollment.connector_id);
    let before = fixture.version(&path);
    assert_eq!(
        registry
            .revoke(request, enrollment.connector_id, enrollment.revision)
            .unwrap(),
        revoked
    );
    assert_eq!(
        registry.revoke(request, enrollment.connector_id, revoked.revision),
        Err(ConnectorError::Conflict)
    );
    assert_eq!(
        registry.revoke(Uuid::new_v4(), enrollment.connector_id, revoked.revision),
        Err(ConnectorError::Conflict)
    );
    assert_eq!(fixture.version(&path), before);
    assert!(
        registry
            .credential_scope(enrollment.connector_id, enrollment.credential)
            .is_err()
    );
    assert!(
        registry
            .publish(activation, published, fixture.admission(), LATER)
            .is_err()
    );
    assert!(registry.catalogue().unwrap().primaries.is_empty());
    assert_eq!(
        registry.snapshots().unwrap()[0].fields().freshness,
        Freshness::Stale
    );
}
